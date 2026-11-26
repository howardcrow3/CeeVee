use super::*;

const VCF: &str = "BEGIN:VCARD\r\nVERSION:3.0\r\nN:Hopper;Grace;;;\r\nFN:Grace Hopper\r\nORG:US Navy;Research\r\nTITLE:Rear Admiral\r\nitem1.EMAIL;TYPE=INTERNET:grace@navy.mil\r\nTEL;TYPE=CELL:555-0101\r\nNOTE:Wrote the first\\ncompiler\r\nEND:VCARD\r\nBEGIN:VCARD\r\nVERSION:3.0\r\nN:Turing;Alan;;;\r\nEMAIL:alan@\r\n example.org\r\nEND:VCARD\r\n";

#[test]
fn parses_vcards_with_groups_folding_and_a_missing_fn() {
    let p = parse_vcards(VCF);
    assert_eq!(p.len(), 2);
    assert_eq!(p[0].full_name, "Grace Hopper");
    assert_eq!(p[0].company.as_deref(), Some("US Navy"));
    assert_eq!(p[0].title.as_deref(), Some("Rear Admiral"));
    assert_eq!(p[0].email.as_deref(), Some("grace@navy.mil"));
    assert_eq!(p[0].phone.as_deref(), Some("555-0101"));
    assert_eq!(p[0].notes.as_deref(), Some("Wrote the first\ncompiler"));
    // Name built from N, and the folded email line joined.
    assert_eq!(p[1].full_name, "Alan Turing");
    assert_eq!(p[1].email.as_deref(), Some("alan@example.org"));
}

#[test]
fn parses_a_google_style_csv_and_a_simple_one() {
    let google = "First Name,Last Name,E-mail 1 - Value,Phone 1 - Value,Organization Name,Organization Title\nAda,Lovelace,ada@example.com,555,Acme,Engineer\n";
    let p = parse_csv(google).unwrap();
    assert_eq!(p[0].full_name, "Ada Lovelace");
    assert_eq!(p[0].email.as_deref(), Some("ada@example.com"));
    assert_eq!(p[0].company.as_deref(), Some("Acme"));
    let simple = "Name,Email\nGrace Hopper,grace@navy.mil\n,orphan@example.com\n";
    let p = parse_csv(simple).unwrap();
    assert_eq!(p.len(), 2);
    assert_eq!(p[0].full_name, "Grace Hopper");
    assert!(parse_csv("a,b\n1,2\n").is_err());
}

#[tokio::test]
async fn saving_twice_matches_instead_of_duplicating_and_skips_nameless_rows() {
    let db = Db::open_in_memory().await.unwrap();
    let people_in = parse_vcards(VCF);
    let first = save(&db, &people_in, "vcard").await.unwrap();
    assert_eq!(
        first,
        Summary {
            added: 2,
            matched: 0,
            skipped: 0
        }
    );
    let second = save(&db, &people_in, "vcard").await.unwrap();
    assert_eq!(second.added, 0);
    assert_eq!(second.matched, 2);
    let nameless = parse_csv("Name,Email\n,x@example.com\n").unwrap();
    assert_eq!(save(&db, &nameless, "csv").await.unwrap().skipped, 1);
    assert_eq!(people::list(&db, None, None).await.unwrap().len(), 2);
}

#[tokio::test]
async fn a_match_fills_blanks_but_never_overwrites() {
    let db = Db::open_in_memory().await.unwrap();
    let id = people::add(
        &db,
        &NewPerson {
            full_name: "Grace Hopper".into(),
            email: Some("grace@navy.mil".into()),
            ..NewPerson::default()
        },
        "manual",
    )
    .await
    .unwrap();
    save(&db, &parse_vcards(VCF), "vcard").await.unwrap();
    let p = people::get(&db, id).await.unwrap();
    assert_eq!(p.phone.as_deref(), Some("555-0101"));
    assert_eq!(p.company.as_deref(), Some("US Navy"));
    assert_eq!(p.source, "manual");
}
