use super::*;

#[test]
fn reads_status_words_the_way_people_write_them() {
    assert_eq!(read_status("Applied"), Status::Sent);
    assert_eq!(read_status("Interviewing"), Status::Interview);
    assert_eq!(read_status("Phone screen"), Status::Screening);
    assert_eq!(read_status("Offer received"), Status::Offer);
    assert_eq!(read_status("Offer declined"), Status::Declined);
    assert_eq!(read_status("Not selected"), Status::Rejected);
    assert_eq!(read_status("Rejected"), Status::Rejected);
    assert_eq!(read_status("Withdrawn"), Status::Withdrawn);
    assert_eq!(read_status("Wishlist"), Status::Draft);
    assert_eq!(read_status(""), Status::Sent);
    assert_eq!(read_status("???"), Status::Sent);
}

#[test]
fn finds_columns_by_name_in_any_order() {
    let csv = "\u{feff}Status,Job Title,Company,Job URL,Extra\nInterviewing,Analyst,Acme,https://x.test/1,hi\n,Planner,Globex,,\n";
    let rows = parse_csv(csv).unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].company, "Acme");
    assert_eq!(rows[0].title, "Analyst");
    assert_eq!(rows[0].status, Status::Interview);
    assert_eq!(rows[0].url.as_deref(), Some("https://x.test/1"));
    assert_eq!(rows[1].status, Status::Sent);
    assert_eq!(rows[1].url, None);
}

#[test]
fn a_file_without_company_and_title_is_refused() {
    let e = parse_csv("Name,Email\nAda,a@x.test\n").unwrap_err();
    assert!(e.to_string().contains("Company"));
}

#[tokio::test]
async fn imports_once_and_leaves_what_is_already_there() {
    let db = Db::open_in_memory().await.unwrap();
    let csv = "Company,Position,Status,Location\nAcme,Analyst,Applied,Columbus\nGlobex,Planner,Rejected,\n,Nobody,Applied,\n";
    let rows = parse_csv(csv).unwrap();
    let a = import(&db, &rows, "2026-11-28T00:00:00Z").await.unwrap();
    assert_eq!(
        a,
        Summary {
            added: 2,
            existing: 0,
            skipped: 1
        }
    );
    let all = tracking::list(&db, &tracking::Filter::default())
        .await
        .unwrap();
    assert_eq!(all.len(), 2);
    assert!(all.iter().any(|x| x.status == Status::Rejected));
    // The same file again adds nothing.
    let b = import(&db, &rows, "2026-11-29T00:00:00Z").await.unwrap();
    assert_eq!(
        b,
        Summary {
            added: 0,
            existing: 2,
            skipped: 1
        }
    );
    assert_eq!(
        tracking::list(&db, &tracking::Filter::default())
            .await
            .unwrap()
            .len(),
        2
    );
}
