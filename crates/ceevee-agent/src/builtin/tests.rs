use super::*;
use ceevee_search::postings::NewPosting;

async fn setup() -> (Db, Registry, i64) {
    let db = Db::open_in_memory().await.unwrap();
    let id = match postings::ingest(
        &db,
        &NewPosting {
            source: "t".into(),
            url: "https://x/1".into(),
            company: "Acme".into(),
            title: "Platform engineer".into(),
            ..NewPosting::default()
        },
        "2026-11-01T00:00:00Z",
    )
    .await
    .unwrap()
    {
        postings::Ingest::New(id) => id,
        other => panic!("{other:?}"),
    };
    let mut r = Registry::new();
    register_all(&mut r, &db).unwrap();
    (db, r, id)
}

#[tokio::test]
async fn lists_and_reads_postings() {
    let (_db, r, id) = setup().await;
    let list = r.call("search.list_jobs", json!({})).await.unwrap();
    assert_eq!(list[0]["title"], "Platform engineer");
    let one = r
        .call("search.get_posting", json!({"id": id}))
        .await
        .unwrap();
    assert!(one["company_id"].is_number());
}

#[tokio::test]
async fn dismiss_hides_and_save_keeps() {
    let (_db, r, id) = setup().await;
    r.call("search.save_posting", json!({"id": id}))
        .await
        .unwrap();
    let list = r.call("search.list_jobs", json!({})).await.unwrap();
    assert_eq!(list[0]["state"], "saved");
    r.call("search.dismiss_posting", json!({"id": id}))
        .await
        .unwrap();
    let list = r.call("search.list_jobs", json!({})).await.unwrap();
    assert_eq!(list.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn bad_input_is_rejected_and_missing_posting_errors() {
    let (_db, r, _id) = setup().await;
    assert!(r
        .call("search.get_posting", json!({"id": 0}))
        .await
        .is_err());
    assert!(r
        .call("search.get_posting", json!({"id": 999}))
        .await
        .is_err());
    assert!(r
        .call("search.list_jobs", json!({"limit": 500}))
        .await
        .is_err());
}

#[tokio::test]
async fn read_tools_flag_untrusted_output() {
    let (_db, r, _id) = setup().await;
    let specs = r.specs_for(&["search.*"]);
    assert_eq!(specs.len(), 4);
    for s in specs {
        assert_eq!(
            s.untrusted_output,
            s.name.contains("list") || s.name.contains("get")
        );
    }
}
