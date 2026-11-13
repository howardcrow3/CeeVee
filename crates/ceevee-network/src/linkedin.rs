//! Import `Connections.csv` from a LinkedIn data export. Each connection becomes a contact with
//! source `linkedin` and a current role at their company.

use ceevee_core::{Error, Result};
use ceevee_db::Db;
use sqlx::Row;

fn db_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Connection {
    pub full_name: String,
    pub profile_url: Option<String>,
    pub email: Option<String>,
    pub company: Option<String>,
    pub position: Option<String>,
    pub connected_on: Option<String>,
}

fn non_empty(s: &str) -> Option<String> {
    let t = s.trim();
    (!t.is_empty()).then(|| t.to_string())
}

/// Parse the export. LinkedIn puts a few lines of notes above the header row, so skip to the
/// line that starts with "First Name". Columns are found by name, so order does not matter.
pub fn parse_connections(csv_text: &str) -> Result<Vec<Connection>> {
    let text = csv_text.trim_start_matches('\u{feff}');
    let start = text
        .split_inclusive('\n')
        .scan(0usize, |off, line| {
            let at = *off;
            *off += line.len();
            Some((at, line))
        })
        .find(|(_, l)| l.trim_start().starts_with("First Name"))
        .map(|(at, _)| at)
        .ok_or_else(|| {
            Error::Invalid("this does not look like LinkedIn's Connections.csv".into())
        })?;
    let mut rdr = csv::ReaderBuilder::new()
        .flexible(true)
        .from_reader(&text.as_bytes()[start..]);
    let headers = rdr
        .headers()
        .map_err(|e| Error::Invalid(e.to_string()))?
        .clone();
    let col = |name: &str| {
        headers
            .iter()
            .position(|h| h.trim().eq_ignore_ascii_case(name))
    };
    let (first, last) = (col("First Name"), col("Last Name"));
    let (url, email) = (col("URL"), col("Email Address"));
    let (company, position, on) = (col("Company"), col("Position"), col("Connected On"));
    let mut out = Vec::new();
    for rec in rdr.records() {
        let rec = rec.map_err(|e| Error::Invalid(e.to_string()))?;
        let get = |i: Option<usize>| i.and_then(|i| rec.get(i)).and_then(non_empty);
        let name = [get(first), get(last)]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" ");
        if name.is_empty() {
            continue;
        }
        out.push(Connection {
            full_name: name,
            profile_url: get(url),
            email: get(email),
            company: get(company),
            position: get(position),
            connected_on: get(on),
        });
    }
    Ok(out)
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct ImportSummary {
    pub added: usize,
    pub updated: usize,
}

/// Save connections. Importing the same export again updates people already saved (matched by
/// profile URL, else by name) instead of duplicating them.
pub async fn import(db: &Db, connections: &[Connection]) -> Result<ImportSummary> {
    let mut summary = ImportSummary::default();
    for c in connections {
        let existing: Option<i64> = match &c.profile_url {
            Some(u) => sqlx::query_scalar("SELECT id FROM contact WHERE profile_url = ?")
                .bind(u)
                .fetch_optional(db.pool())
                .await
                .map_err(db_err)?,
            None => sqlx::query_scalar(
                "SELECT id FROM contact WHERE full_name = ? AND source = 'linkedin' AND profile_url IS NULL",
            )
            .bind(&c.full_name)
            .fetch_optional(db.pool())
            .await
            .map_err(db_err)?,
        };
        let id = match existing {
            Some(id) => {
                sqlx::query("UPDATE contact SET email = COALESCE(?, email) WHERE id = ?")
                    .bind(&c.email)
                    .bind(id)
                    .execute(db.pool())
                    .await
                    .map_err(db_err)?;
                summary.updated += 1;
                id
            }
            None => {
                summary.added += 1;
                sqlx::query_scalar(
                    "INSERT INTO contact(full_name, email, profile_url, source) VALUES (?, ?, ?, 'linkedin') RETURNING id",
                )
                .bind(&c.full_name)
                .bind(&c.email)
                .bind(&c.profile_url)
                .fetch_one(db.pool())
                .await
                .map_err(db_err)?
            }
        };
        if let Some(company_name) = &c.company {
            let company = ceevee_search::companies::find_or_create(db, company_name).await?;
            sqlx::query("DELETE FROM contact_role WHERE contact_id = ? AND is_current = 1")
                .bind(id)
                .execute(db.pool())
                .await
                .map_err(db_err)?;
            sqlx::query(
                "INSERT INTO contact_role(contact_id, company_id, title, is_current) VALUES (?, ?, ?, 1)",
            )
            .bind(id)
            .bind(company.id)
            .bind(&c.position)
            .execute(db.pool())
            .await
            .map_err(db_err)?;
        }
    }
    Ok(summary)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnownPerson {
    pub contact_id: i64,
    pub full_name: String,
    pub title: Option<String>,
    pub profile_url: Option<String>,
}

/// People the user knows who currently work at a company.
pub async fn connections_at(db: &Db, company_id: i64) -> Result<Vec<KnownPerson>> {
    let rows = sqlx::query(
        "SELECT c.id, c.full_name, r.title, c.profile_url FROM contact c \
         JOIN contact_role r ON r.contact_id = c.id \
         WHERE r.company_id = ? AND r.is_current = 1 ORDER BY c.full_name",
    )
    .bind(company_id)
    .fetch_all(db.pool())
    .await
    .map_err(db_err)?;
    Ok(rows
        .iter()
        .map(|r| KnownPerson {
            contact_id: r.get(0),
            full_name: r.get(1),
            title: r.get(2),
            profile_url: r.get(3),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXPORT: &str = "Notes:\r\n\"When exporting your connection data, you may notice...\"\n\nFirst Name,Last Name,URL,Email Address,Company,Position,Connected On\nAda,Lovelace,https://www.linkedin.com/in/ada,,Acme Inc.,Engineer,10 Nov 2026\nGrace,Hopper,https://www.linkedin.com/in/grace,grace@navy.mil,\"Acme, Inc\",\"VP, Engineering\",01 Oct 2026\nAlan,,,,,,\n,,,,,,\n";

    #[test]
    fn parses_the_export_past_the_notes() {
        let c = parse_connections(EXPORT).unwrap();
        assert_eq!(c.len(), 3);
        assert_eq!(c[0].full_name, "Ada Lovelace");
        assert_eq!(c[0].email, None);
        assert_eq!(c[1].position.as_deref(), Some("VP, Engineering"));
        assert_eq!(c[2].full_name, "Alan");
    }

    #[test]
    fn rejects_other_files() {
        assert!(parse_connections("a,b\n1,2\n").is_err());
    }

    #[tokio::test]
    async fn import_links_people_to_companies_and_is_repeatable() {
        let db = Db::open_in_memory().await.unwrap();
        let c = parse_connections(EXPORT).unwrap();
        let first = import(&db, &c).await.unwrap();
        assert_eq!(
            first,
            ImportSummary {
                added: 3,
                updated: 0
            }
        );
        let second = import(&db, &c).await.unwrap();
        assert_eq!(
            second,
            ImportSummary {
                added: 0,
                updated: 3
            }
        );

        let acme = ceevee_search::companies::find_or_create(&db, "Acme")
            .await
            .unwrap();
        let people = connections_at(&db, acme.id).await.unwrap();
        // "Acme Inc." and "Acme, Inc" are the same company.
        assert_eq!(people.len(), 2);
        assert_eq!(people[0].full_name, "Ada Lovelace");
        assert_eq!(people[1].title.as_deref(), Some("VP, Engineering"));
    }
}
