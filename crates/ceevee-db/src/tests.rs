use super::*;

async fn count(db: &Db, sql: &str) -> i64 {
    sqlx::query_scalar::<_, i64>(sql)
        .fetch_one(db.pool())
        .await
        .unwrap()
}

#[tokio::test]
async fn migrations_create_all_core_tables() {
    let db = Db::open_in_memory().await.unwrap();
    assert_eq!(db.schema_version().await.unwrap(), 2);
    let tables = count(
        &db,
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' \
         AND name NOT LIKE '%_fts%' AND name <> '_sqlx_migrations'",
    )
    .await;
    assert_eq!(tables, 37);
}

#[tokio::test]
async fn jobs_query_by_category_and_company() {
    let db = Db::open_in_memory().await.unwrap();
    let p = db.pool();
    sqlx::query("INSERT INTO company(name) VALUES ('Acme')")
        .execute(p)
        .await
        .unwrap();
    sqlx::query("INSERT INTO job_category(name) VALUES ('Staff engineer')")
        .execute(p)
        .await
        .unwrap();
    sqlx::query("INSERT INTO posting(source,url,company_id,title,body) VALUES ('greenhouse','https://x/1',1,'Rust engineer','Build tauri apps')")
        .execute(p).await.unwrap();
    sqlx::query("INSERT INTO posting_category(posting_id,category_id,score) VALUES (1,1,88.0)")
        .execute(p)
        .await
        .unwrap();
    assert_eq!(
        count(&db, "SELECT COUNT(*) FROM posting WHERE company_id = 1").await,
        1
    );
    assert_eq!(
        count(
            &db,
            "SELECT COUNT(*) FROM posting_category WHERE category_id = 1 AND score > 70"
        )
        .await,
        1
    );
}

#[tokio::test]
async fn full_text_search_follows_inserts_updates_and_deletes() {
    let db = Db::open_in_memory().await.unwrap();
    let p = db.pool();
    sqlx::query("INSERT INTO posting(source,url,title,body) VALUES ('s','u','Rust engineer','tauri desktop')").execute(p).await.unwrap();
    assert_eq!(
        count(
            &db,
            "SELECT COUNT(*) FROM posting_fts WHERE posting_fts MATCH 'tauri'"
        )
        .await,
        1
    );
    sqlx::query("UPDATE posting SET body = 'sqlite only'")
        .execute(p)
        .await
        .unwrap();
    assert_eq!(
        count(
            &db,
            "SELECT COUNT(*) FROM posting_fts WHERE posting_fts MATCH 'tauri'"
        )
        .await,
        0
    );
    assert_eq!(
        count(
            &db,
            "SELECT COUNT(*) FROM posting_fts WHERE posting_fts MATCH 'sqlite'"
        )
        .await,
        1
    );
    sqlx::query("DELETE FROM posting").execute(p).await.unwrap();
    assert_eq!(
        count(
            &db,
            "SELECT COUNT(*) FROM posting_fts WHERE posting_fts MATCH 'sqlite'"
        )
        .await,
        0
    );
}

#[tokio::test]
async fn foreign_keys_are_enforced() {
    let db = Db::open_in_memory().await.unwrap();
    let r =
        sqlx::query("INSERT INTO posting_category(posting_id,category_id,score) VALUES (9,9,1.0)")
            .execute(db.pool())
            .await;
    assert!(r.is_err());
}

#[tokio::test]
async fn file_database_uses_wal_and_reopens_without_backup() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ceevee.db");
    {
        let db = Db::open(&path).await.unwrap();
        let mode: String = sqlx::query_scalar("PRAGMA journal_mode")
            .fetch_one(db.pool())
            .await
            .unwrap();
        assert_eq!(mode, "wal");
        db.pool().close().await;
    }
    let db = Db::open(&path).await.unwrap();
    assert_eq!(db.schema_version().await.unwrap(), 2);
    let backups = std::fs::read_dir(dir.path())
        .unwrap()
        .filter(|e| {
            e.as_ref()
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains(".bak-")
        })
        .count();
    assert_eq!(backups, 0, "no backup when nothing is pending");
}

#[tokio::test]
async fn pending_migration_triggers_backup() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ceevee.db");
    // Simulate a database one migration behind: apply only 0001 by hand.
    {
        let pool = connect(path.to_str().unwrap(), 1).await.unwrap();
        let first = MIGRATOR.iter().next().unwrap();
        sqlx::raw_sql(&first.sql).execute(&pool).await.unwrap();
        sqlx::query("CREATE TABLE _sqlx_migrations (version BIGINT PRIMARY KEY)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO _sqlx_migrations(version) VALUES (?)")
            .bind(first.version)
            .execute(&pool)
            .await
            .unwrap();
        pool.close().await;
    }
    assert!(Db::has_pending_migrations(&path).await.unwrap());
    let backup = backup_file(&path).unwrap();
    assert!(backup.exists());
}

#[tokio::test]
async fn settings_round_trip_and_overwrite() {
    let db = Db::open_in_memory().await.unwrap();
    assert_eq!(db.get_setting("a").await.unwrap(), None);
    db.set_setting("a", "1").await.unwrap();
    db.set_setting("a", "2").await.unwrap();
    assert_eq!(db.get_setting("a").await.unwrap().as_deref(), Some("2"));
}
