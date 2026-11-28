//! IMAP against a tiny in-process server that speaks just enough of the protocol.

use std::sync::{Arc, Mutex};

use ceevee_db::Db;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

use crate::imap::{imap_date, ImapConfig, SyncState};
use crate::{store, sync};

struct Server {
    port: u16,
    /// Every command line the client sent.
    log: Arc<Mutex<Vec<String>>>,
}

fn mail(from: &str, subject: &str, body: &str, id: &str) -> Vec<u8> {
    format!(
        "From: Dana <{from}>\r\nTo: me@example.org\r\nSubject: {subject}\r\nMessage-ID: <{id}>\r\n\
         Date: Fri, 13 Nov 2026 10:00:00 +0000\r\n\r\n{body}\r\n"
    )
    .into_bytes()
}

fn split(raw: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let at = raw.windows(4).position(|w| w == b"\r\n\r\n").unwrap() + 4;
    (raw[..at].to_vec(), raw.to_vec())
}

async fn serve(messages: Vec<(u32, Vec<u8>)>, password: &'static str) -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let log = Arc::new(Mutex::new(vec![]));
    let log2 = log.clone();
    tokio::spawn(async move {
        loop {
            let Ok((sock, _)) = listener.accept().await else {
                return;
            };
            let messages = messages.clone();
            let log = log2.clone();
            tokio::spawn(async move {
                let (r, mut w) = sock.into_split();
                let mut lines = BufReader::new(r).lines();
                w.write_all(b"* OK ready\r\n").await.unwrap();
                while let Ok(Some(line)) = lines.next_line().await {
                    log.lock().unwrap().push(line.clone());
                    let (tag, cmd) = line.split_once(' ').unwrap();
                    let up = cmd.to_ascii_uppercase();
                    if up.starts_with("LOGIN") {
                        let ok = cmd.contains(password);
                        let reply = if ok {
                            format!("{tag} OK done\r\n")
                        } else {
                            format!("{tag} NO [AUTHENTICATIONFAILED] Invalid credentials\r\n")
                        };
                        w.write_all(reply.as_bytes()).await.unwrap();
                    } else if up.starts_with("SELECT") {
                        let next = messages.last().map_or(1, |m| m.0 + 1);
                        w.write_all(
                            format!("* {} EXISTS\r\n* OK [UIDVALIDITY 7] ok\r\n* OK [UIDNEXT {next}] ok\r\n{tag} OK [READ-WRITE] done\r\n", messages.len()).as_bytes(),
                        )
                        .await
                        .unwrap();
                    } else if up.starts_with("UID SEARCH") {
                        // "UID n:*" returns at least the last message, like a real server.
                        let from = up
                            .strip_prefix("UID SEARCH UID ")
                            .and_then(|r| r.split(':').next())
                            .and_then(|n| n.parse::<u32>().ok());
                        let ids: Vec<String> = messages
                            .iter()
                            .filter(|m| {
                                from.is_none_or(|f| m.0 >= f)
                                    || Some(m.0) == messages.last().map(|l| l.0)
                            })
                            .map(|m| m.0.to_string())
                            .collect();
                        w.write_all(
                            format!("* SEARCH {}\r\n{tag} OK done\r\n", ids.join(" ")).as_bytes(),
                        )
                        .await
                        .unwrap();
                    } else if up.starts_with("UID FETCH") {
                        let set = cmd.split_whitespace().nth(2).unwrap();
                        let want_body = up.contains("BODY.PEEK[]");
                        for id in set.split(',').map(|n| n.parse::<u32>().unwrap()) {
                            let Some((_, raw)) = messages.iter().find(|m| m.0 == id) else {
                                continue;
                            };
                            let (head, full) = split(raw);
                            let (name, data) = if want_body {
                                ("BODY[]", full)
                            } else {
                                ("BODY[HEADER]", head)
                            };
                            w.write_all(
                                format!("* {id} FETCH (UID {id} {name} {{{}}}\r\n", data.len())
                                    .as_bytes(),
                            )
                            .await
                            .unwrap();
                            w.write_all(&data).await.unwrap();
                            w.write_all(b")\r\n").await.unwrap();
                        }
                        w.write_all(format!("{tag} OK done\r\n").as_bytes())
                            .await
                            .unwrap();
                    } else if up.starts_with("LOGOUT") {
                        w.write_all(format!("* BYE\r\n{tag} OK done\r\n").as_bytes())
                            .await
                            .unwrap();
                        return;
                    } else {
                        w.write_all(format!("{tag} OK done\r\n").as_bytes())
                            .await
                            .unwrap();
                    }
                }
            });
        }
    });
    Server { port, log }
}

fn config(port: u16) -> serde_json::Value {
    serde_json::to_value(ImapConfig {
        host: "127.0.0.1".into(),
        port,
        username: "me@example.org".into(),
        tls: false,
    })
    .unwrap()
}

#[test]
fn dates_for_the_first_sync() {
    assert_eq!(imap_date(1_794_564_000, 0), "13-Nov-2026");
    assert_eq!(imap_date(1_794_564_000, 90), "15-Aug-2026");
}

#[tokio::test]
async fn a_sync_downloads_only_employer_mail_and_does_not_repeat() {
    let server = serve(
        vec![
            (
                1,
                mail(
                    "dana@acme.com",
                    "Interview",
                    "Can we schedule an interview Tuesday?",
                    "i1@acme",
                ),
            ),
            (
                2,
                mail("friend@gmail.com", "Dinner", "Free Friday?", "f1@x"),
            ),
            (
                3,
                mail(
                    "noreply@greenhouse.io",
                    "Thanks",
                    "Thank you for applying.",
                    "g1@gh",
                ),
            ),
        ],
        "app-password",
    )
    .await;
    let db = Db::open_in_memory().await.unwrap();
    let acct = store::add_account(&db, "imap", "me@example.org", &config(server.port), "k")
        .await
        .unwrap();
    sqlx::query("INSERT INTO company(name, email_domains) VALUES ('Acme', '[\"acme.com\"]')")
        .execute(db.pool())
        .await
        .unwrap();

    let s = sync::sync_imap(
        &db,
        acct.id,
        "app-password",
        || None,
        1_794_564_000,
        "2026-11-13T10:00:00Z",
    )
    .await
    .unwrap();
    assert_eq!(s.seen, 3);
    assert_eq!(s.employer, 2);
    assert_eq!(
        store::list_messages(&db, &Default::default())
            .await
            .unwrap()
            .len(),
        2
    );

    // Every message not sent by the account is read, because a recruiter can write from any
    // address; the friend's dinner mail is read, found not to be about a job, and not stored.
    let log = server.log.lock().unwrap().clone();
    let body_fetches: Vec<&String> = log.iter().filter(|l| l.contains("BODY.PEEK[]")).collect();
    assert_eq!(body_fetches.len(), 1);
    assert!(body_fetches[0].contains("1,2,3"), "{body_fetches:?}");
    assert!(log
        .iter()
        .all(|l| !l.to_ascii_uppercase().contains("STORE")));

    // Second sync starts after the last uid and finds nothing new.
    let state = store::get_account(&db, acct.id).await.unwrap().sync_state;
    assert_eq!(
        serde_json::from_value::<SyncState>(state).unwrap().last_uid,
        Some(3)
    );
    let s2 = sync::sync_imap(
        &db,
        acct.id,
        "app-password",
        || None,
        1_794_564_100,
        "2026-11-13T10:01:40Z",
    )
    .await
    .unwrap();
    assert_eq!(s2.employer, 0);
    assert_eq!(
        store::list_messages(&db, &Default::default())
            .await
            .unwrap()
            .len(),
        2
    );
}

#[tokio::test]
async fn a_wrong_password_gives_a_plain_message() {
    let server = serve(vec![], "right").await;
    let cfg: ImapConfig = serde_json::from_value(config(server.port)).unwrap();
    let err = crate::imap::test_login(&cfg, "wrong").await.unwrap_err();
    assert!(crate::imap::explain(&err).contains("app password"), "{err}");
    crate::imap::test_login(&cfg, "right").await.unwrap();
}

#[tokio::test]
async fn an_unreachable_server_is_explained() {
    let cfg = ImapConfig {
        host: "127.0.0.1".into(),
        port: 1,
        username: "a".into(),
        tls: false,
    };
    let err = crate::imap::test_login(&cfg, "x").await.unwrap_err();
    assert!(
        crate::imap::explain(&err).contains("could not reach"),
        "{err}"
    );
}

#[test]
fn presets_cover_the_common_providers() {
    use crate::imap::preset_for;
    assert_eq!(preset_for("a@gmail.com"), Some(("imap.gmail.com", 993)));
    assert_eq!(
        preset_for("a@Outlook.com"),
        Some(("outlook.office365.com", 993))
    );
    assert_eq!(preset_for("a@example.org"), None);
}

mod gmail {

    use ceevee_core::Result;
    use ceevee_db::Db;
    use ceevee_google::MailApi;
    use serde_json::json;

    use super::mail;
    use crate::{store, sync};

    #[derive(Default)]
    struct Fake {
        mails: Vec<(String, Vec<u8>)>,
        queries: std::sync::Mutex<Vec<String>>,
        raw_calls: std::sync::Mutex<Vec<String>>,
        /// Downloading this message fails, as a 403 would.
        fail_raw: Option<String>,
    }

    impl MailApi for Fake {
        async fn list_ids(&self, q: &str, _max: usize) -> Result<Vec<String>> {
            self.queries.lock().unwrap().push(q.into());
            Ok(self.mails.iter().map(|m| m.0.clone()).collect())
        }
        async fn headers(&self, id: &str) -> Result<Vec<u8>> {
            let raw = &self.mails.iter().find(|m| m.0 == id).unwrap().1;
            let at = raw.windows(4).position(|w| w == b"\r\n\r\n").unwrap() + 4;
            Ok(raw[..at].to_vec())
        }
        async fn raw(&self, id: &str) -> Result<Vec<u8>> {
            self.raw_calls.lock().unwrap().push(id.into());
            if self.fail_raw.as_deref() == Some(id) {
                return Err(ceevee_core::Error::Invalid("403 Forbidden".into()));
            }
            Ok(self.mails.iter().find(|m| m.0 == id).unwrap().1.clone())
        }
    }

    #[tokio::test]
    async fn gmail_sync_moves_a_hand_added_application_for_mail_from_a_personal_address() {
        use ceevee_search::tracking::{self, Status};
        let api = Fake {
            mails: vec![(
                "d".into(),
                mail(
                    "someone@gmail.com",
                    "Interview invitation: Product Manager at Acme Robotics",
                    "Hi, thank you for applying. We would like to interview you next week for the Product Manager role at Acme Robotics. Please reply with times that work. Thanks, Acme Robotics recruiting",
                    "demo@gmail",
                ),
            )],
            ..Default::default()
        };
        let db = Db::open_in_memory().await.unwrap();
        let acct = store::add_account(&db, "gmail", "me@gmail.com", &json!({}), "k")
            .await
            .unwrap();
        let app = tracking::add_manual(
            &db,
            "Acme Robotics",
            "Product Manager",
            Status::Sent,
            "2026-11-13T09:00:00Z",
        )
        .await
        .unwrap();
        let s = sync::sync_gmail(
            &db,
            acct.id,
            &api,
            || None,
            1_794_564_000,
            "2026-11-13T10:00:00Z",
        )
        .await
        .unwrap();
        assert_eq!((s.seen, s.employer, s.status_changes), (1, 1, 1));
        assert_eq!(
            tracking::get(&db, app.id).await.unwrap().status,
            Status::Interview
        );
    }

    #[tokio::test]
    async fn a_gap_longer_than_one_sync_is_worked_through_oldest_first() {
        // 501 messages, newest first as Gmail lists them: "m0" is the newest.
        let mails: Vec<(String, Vec<u8>)> = (0..501)
            .map(|i| {
                (
                    format!("g{i}"),
                    mail(
                        "news@example.org",
                        "Weekly digest",
                        "Nothing here.",
                        &format!("m{i}@x"),
                    ),
                )
            })
            .collect();
        let api = Fake {
            mails,
            ..Default::default()
        };
        let db = Db::open_in_memory().await.unwrap();
        let acct = store::add_account(&db, "gmail", "me@gmail.com", &json!({}), "k")
            .await
            .unwrap();
        let now = 1_794_564_000 + 10 * 86_400;
        sync::sync_gmail(&db, acct.id, &api, || None, now, "2026-11-23T10:00:00Z")
            .await
            .unwrap();
        // The oldest 500 were handled; the newest is left for the next sync.
        assert!(store::is_seen(&db, acct.id, "m500@x").await.unwrap());
        assert!(!store::is_seen(&db, acct.id, "m0@x").await.unwrap());
        // The window moved only as far as the mail handled, not to "now".
        let st = store::get_account(&db, acct.id).await.unwrap().sync_state;
        assert_eq!(st["after"], 1_794_564_000_i64, "{st}");
    }

    #[tokio::test]
    async fn a_failed_download_does_not_cost_the_mail_already_handled() {
        use ceevee_search::tracking::{self, Status};
        let body = "We would like to interview you for the role at Acme Robotics.";
        let api = Fake {
            mails: vec![
                (
                    "a".into(),
                    mail("x@gmail.com", "Interview at Acme Robotics", body, "f1@x"),
                ),
                (
                    "b".into(),
                    mail("y@gmail.com", "Interview at Acme Robotics", body, "f2@x"),
                ),
            ],
            fail_raw: Some("b".into()),
            ..Default::default()
        };
        let db = Db::open_in_memory().await.unwrap();
        let acct = store::add_account(&db, "gmail", "me@gmail.com", &json!({}), "k")
            .await
            .unwrap();
        let app = tracking::add_manual(
            &db,
            "Acme Robotics",
            "PM",
            Status::Sent,
            "2026-11-13T09:00:00Z",
        )
        .await
        .unwrap();
        let s = sync::sync_gmail(
            &db,
            acct.id,
            &api,
            || None,
            1_794_564_000,
            "2026-11-13T10:00:00Z",
        )
        .await
        .unwrap();
        assert_eq!(s.status_changes, 1);
        assert_eq!(
            tracking::get(&db, app.id).await.unwrap().status,
            Status::Interview
        );
        // The window did not advance, so the failed message is tried again.
        let st = store::get_account(&db, acct.id).await.unwrap().sync_state;
        assert!(st["after"].is_null(), "{st}");
    }

    #[tokio::test]
    async fn mail_set_aside_by_older_rules_is_looked_at_again() {
        use ceevee_search::tracking::{self, Status};
        let api = Fake {
            mails: vec![(
                "d".into(),
                mail(
                    "someone@gmail.com",
                    "Interview invitation: Product Manager at Acme Robotics",
                    "We would like to interview you next week for the Product Manager role. Thanks, recruiting",
                    "demo2@gmail",
                ),
            )],
            ..Default::default()
        };
        let db = Db::open_in_memory().await.unwrap();
        let acct = store::add_account(&db, "gmail", "me@gmail.com", &json!({}), "k")
            .await
            .unwrap();
        // An older build set the message aside and recorded an old rules version.
        store::mark_seen(&db, acct.id, "<demo2@gmail>")
            .await
            .unwrap();
        store::set_sync_state(
            &db,
            acct.id,
            &json!({"after": 1_794_000_000_i64}),
            "2026-11-13T09:00:00Z",
        )
        .await
        .unwrap();
        let app = tracking::add_manual(
            &db,
            "Acme Robotics",
            "Product Manager",
            Status::Sent,
            "2026-11-13T09:00:00Z",
        )
        .await
        .unwrap();
        let s = sync::sync_gmail(
            &db,
            acct.id,
            &api,
            || None,
            1_794_564_000,
            "2026-11-13T10:00:00Z",
        )
        .await
        .unwrap();
        assert_eq!(s.status_changes, 1);
        assert_eq!(
            tracking::get(&db, app.id).await.unwrap().status,
            Status::Interview
        );
    }

    #[tokio::test]
    async fn gmail_sync_stores_only_job_mail_and_does_not_read_set_aside_mail_again() {
        let api = Fake {
            mails: vec![
                (
                    "a".into(),
                    mail(
                        "dana@acme.com",
                        "Interview",
                        "Can we schedule an interview?",
                        "g1@acme",
                    ),
                ),
                ("b".into(), mail("mom@gmail.com", "Hi", "Call me", "g2@x")),
            ],
            ..Default::default()
        };
        let db = Db::open_in_memory().await.unwrap();
        let acct = store::add_account(&db, "gmail", "me@gmail.com", &json!({}), "k")
            .await
            .unwrap();
        sqlx::query("INSERT INTO company(name, email_domains) VALUES ('Acme', '[\"acme.com\"]')")
            .execute(db.pool())
            .await
            .unwrap();
        let s = sync::sync_gmail(
            &db,
            acct.id,
            &api,
            || None,
            1_794_564_000,
            "2026-11-13T10:00:00Z",
        )
        .await
        .unwrap();
        assert_eq!((s.seen, s.employer), (2, 1));
        // Only mail that hints at a job search is downloaded.
        assert_eq!(api.raw_calls.lock().unwrap().as_slice(), ["a"]);
        assert!(api.queries.lock().unwrap()[0].contains("newer_than:90d"));
        assert!(api.queries.lock().unwrap()[0].contains("-category:promotions"));
        sync::sync_gmail(
            &db,
            acct.id,
            &api,
            || None,
            1_794_564_100,
            "2026-11-13T10:01:40Z",
        )
        .await
        .unwrap();
        assert!(
            api.queries.lock().unwrap()[1].ends_with(&format!("after:{}", 1_794_564_000 - 86_400)),
            "{:?}",
            api.queries.lock().unwrap()
        );
        // The day of overlap lists both again, but neither is downloaded a second time.
        assert_eq!(api.raw_calls.lock().unwrap().as_slice(), ["a"]);
        assert_eq!(
            store::list_messages(&db, &Default::default())
                .await
                .unwrap()
                .len(),
            1
        );
    }
}

/// Needs the internet: a real TLS handshake with a real provider, ending at the sign-in refusal.
/// Run with `cargo test -p ceevee-mail -- --ignored`.
#[tokio::test]
#[ignore]
async fn a_real_provider_over_tls_refuses_a_bad_password_in_plain_words() {
    let cfg = ImapConfig {
        host: "imap.gmail.com".into(),
        port: 993,
        username: "nobody-ceevee-test@gmail.com".into(),
        tls: true,
    };
    let err = crate::imap::test_login(&cfg, "not-a-password")
        .await
        .unwrap_err();
    assert!(crate::imap::explain(&err).contains("app password"), "{err}");
}
