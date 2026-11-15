//! Glue: reads and writes the stored facts and calls the server.

use ceevee_db::Db;
use chrono::{DateTime, Utc};

use crate::anchor::Anchor;
use crate::api::Api;
use crate::status::{self, Evaluation, Facts, Problem, Status};
use crate::token::{Claims, Verifier};
use crate::{key, LicenseError, Result};

const TRIAL_TOKEN_SETTING: &str = "license.trial_token";

fn db_err(e: impl std::fmt::Display) -> LicenseError {
    ceevee_core::Error::Database(e.to_string()).into()
}

/// What the UI shows.
#[derive(Debug, Clone)]
pub struct Report {
    pub status: Status,
    pub problem: Option<Problem>,
    pub machine: String,
}

pub struct LicenseService<A: Api, K: Anchor> {
    db: Db,
    api: A,
    anchor: K,
    verifier: Option<Verifier>,
    machine: String,
    app_version: String,
    app_major: u32,
}

impl<A: Api, K: Anchor> LicenseService<A, K> {
    pub fn new(
        db: Db,
        api: A,
        anchor: K,
        verifier: Option<Verifier>,
        machine: String,
        app_version: &str,
    ) -> Self {
        let app_major = app_version
            .split('.')
            .next()
            .and_then(|m| m.parse().ok())
            .unwrap_or(0);
        Self {
            db,
            api,
            anchor,
            verifier,
            machine,
            app_version: app_version.to_string(),
            app_major,
        }
    }

    async fn load(&self) -> Result<Facts> {
        let row: Option<(Option<String>, Option<String>, Option<String>)> =
            sqlx::query_as("SELECT trial_started_at, license_blob, last_checked_at FROM license_state WHERE id = 1")
                .fetch_optional(self.db.pool())
                .await
                .map_err(db_err)?;
        let (started, blob, seen) = row.unwrap_or_default();
        let parse = |s: Option<String>| s.and_then(|s| s.parse::<DateTime<Utc>>().ok());
        Ok(Facts {
            db_trial_start: parse(started),
            anchor_trial_start: self.anchor.get(),
            trial_token: self.db.get_setting(TRIAL_TOKEN_SETTING).await?,
            license_blob: blob,
            last_seen: parse(seen),
        })
    }

    async fn save(
        &self,
        started: Option<DateTime<Utc>>,
        blob: Option<&str>,
        seen: DateTime<Utc>,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO license_state (id, trial_started_at, license_blob, last_checked_at) VALUES (1, ?1, ?2, ?3)
             ON CONFLICT(id) DO UPDATE SET
               trial_started_at = COALESCE(?1, trial_started_at),
               license_blob = ?2,
               last_checked_at = ?3",
        )
        .bind(started.map(|d| d.to_rfc3339()))
        .bind(blob)
        .bind(seen.to_rfc3339())
        .execute(self.db.pool())
        .await
        .map_err(db_err)?;
        Ok(())
    }

    fn evaluate(&self, facts: &Facts, now: DateTime<Utc>) -> Evaluation {
        status::evaluate(
            facts,
            now,
            &self.machine,
            self.app_major,
            self.verifier.as_ref(),
        )
    }

    /// Where the person stands now. The first call on a computer starts the trial;
    /// every call moves the "newest time seen" forward and keeps both copies of the
    /// trial start in step. Needs no network.
    pub async fn status(&self, now: DateTime<Utc>) -> Result<Report> {
        let facts = self.load().await?;
        let mut eval = self.evaluate(&facts, now);
        let mut facts = facts;
        if eval.trial_start.is_none() && !matches!(eval.status, Status::Licensed { .. }) {
            facts.db_trial_start = Some(now);
            eval = self.evaluate(&facts, now);
        }
        let seen = status::effective_now(now, facts.last_seen);
        let start = eval.trial_start;
        self.save(start, facts.license_blob.as_deref(), seen)
            .await?;
        if let Some(s) = start {
            if facts.anchor_trial_start != Some(s) {
                self.anchor.set(s);
            }
        }
        Ok(Report {
            status: eval.status,
            problem: eval.problem,
            machine: self.machine.clone(),
        })
    }

    /// Ask turtlebox.studio for a signed trial token once. Offline is fine: this is
    /// retried on a later launch. Returns whether a token is now stored.
    pub async fn fetch_trial_token(&self) -> bool {
        if self
            .db
            .get_setting(TRIAL_TOKEN_SETTING)
            .await
            .ok()
            .flatten()
            .is_some()
        {
            return true;
        }
        let Ok(token) = self.api.trial_token(&self.machine).await else {
            return false;
        };
        match self.verifier.as_ref().map(|v| v.verify(&token)) {
            Some(Ok(Claims::Trial { machine, .. })) if machine == self.machine => self
                .db
                .set_setting(TRIAL_TOKEN_SETTING, &token)
                .await
                .is_ok(),
            _ => false,
        }
    }

    /// Activate a key on this computer.
    pub async fn activate(&self, input: &str, now: DateTime<Utc>) -> Result<Report> {
        let verifier = self.verifier.as_ref().ok_or(LicenseError::NoPublicKey)?;
        let key = key::normalize(input)?;
        let blob = self
            .api
            .activate(&key, &self.machine, &self.app_version)
            .await?;
        match verifier.verify(&blob)? {
            Claims::License {
                key: k,
                machine,
                major,
                ..
            } => {
                if k != key {
                    return Err(LicenseError::BadSignature);
                }
                if machine != self.machine {
                    return Err(LicenseError::WrongMachine);
                }
                if major < self.app_major {
                    return Err(LicenseError::WrongVersion);
                }
            }
            Claims::Trial { .. } => return Err(LicenseError::BadSignature),
        }
        let facts = self.load().await?;
        let seen = status::effective_now(now, facts.last_seen);
        self.save(None, Some(&blob), seen).await?;
        self.status(now).await
    }

    /// Free this computer's seat and remove the license here. The server is asked
    /// first, so a failure leaves the license in place rather than losing the seat.
    pub async fn deactivate(&self, now: DateTime<Utc>) -> Result<Report> {
        let facts = self.load().await?;
        if let Status::Licensed { key, .. } = self.evaluate(&facts, now).status {
            self.api.deactivate(&key, &self.machine).await?;
        }
        let seen = status::effective_now(now, facts.last_seen);
        sqlx::query(
            "UPDATE license_state SET license_blob = NULL, last_checked_at = ?1 WHERE id = 1",
        )
        .bind(seen.to_rfc3339())
        .execute(self.db.pool())
        .await
        .map_err(db_err)?;
        self.status(now).await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::anchor::MemoryAnchor;
    use crate::token::{sign, testing::keypair};
    use ed25519_dalek::SigningKey;

    fn t(s: &str) -> DateTime<Utc> {
        s.parse().unwrap()
    }

    const KEY: &str = "CEEV-7KQ2M-9XB4D-TR5VW-H3NZ8";

    struct FakeApi {
        sk: SigningKey,
        machine: String,
        fail: Mutex<Option<fn() -> LicenseError>>,
        deactivated: Mutex<Vec<String>>,
        major: u32,
    }

    impl FakeApi {
        fn new(sk: SigningKey, machine: &str) -> Self {
            Self {
                sk,
                machine: machine.into(),
                fail: Mutex::new(None),
                deactivated: Mutex::default(),
                major: 1,
            }
        }
        fn failing(&self) -> Option<LicenseError> {
            self.fail.lock().unwrap().map(|f| f())
        }
    }

    impl Api for FakeApi {
        async fn trial_token(&self, machine: &str) -> Result<String> {
            if let Some(e) = self.failing() {
                return Err(e);
            }
            Ok(sign(
                &self.sk,
                &Claims::Trial {
                    machine: machine.into(),
                    started_at: t("2026-12-01T00:00:00Z"),
                },
            ))
        }
        async fn activate(&self, key: &str, _machine: &str, _v: &str) -> Result<String> {
            if let Some(e) = self.failing() {
                return Err(e);
            }
            Ok(sign(
                &self.sk,
                &Claims::License {
                    key: key.into(),
                    machine: self.machine.clone(),
                    issued_at: t("2026-12-05T00:00:00Z"),
                    major: self.major,
                    seats: 2,
                },
            ))
        }
        async fn deactivate(&self, key: &str, _m: &str) -> Result<()> {
            if let Some(e) = self.failing() {
                return Err(e);
            }
            self.deactivated.lock().unwrap().push(key.into());
            Ok(())
        }
    }

    async fn svc(machine: &str) -> LicenseService<FakeApi, MemoryAnchor> {
        let (sk, v) = keypair();
        LicenseService::new(
            Db::open_in_memory().await.unwrap(),
            FakeApi::new(sk, machine),
            MemoryAnchor::default(),
            Some(v),
            machine.into(),
            "1.0.0",
        )
    }

    #[tokio::test]
    async fn first_run_starts_a_14_day_trial_and_keeps_both_copies() {
        let s = svc("m").await;
        let r = s.status(t("2026-12-01T09:00:00Z")).await.unwrap();
        assert!(matches!(r.status, Status::Trial { days_left: 14, .. }));
        assert_eq!(s.anchor.get(), Some(t("2026-12-01T09:00:00Z")));
        // Later launches keep counting from the same start.
        let r = s.status(t("2026-12-08T09:00:00Z")).await.unwrap();
        assert!(matches!(r.status, Status::Trial { days_left: 7, .. }));
        let r = s.status(t("2026-12-15T09:00:00Z")).await.unwrap();
        assert!(r.status.read_only());
    }

    #[tokio::test]
    async fn clearing_the_database_does_not_restart_the_trial_because_of_the_keychain() {
        let s = svc("m").await;
        s.status(t("2026-12-01T09:00:00Z")).await.unwrap();
        sqlx::query("DELETE FROM license_state")
            .execute(s.db.pool())
            .await
            .unwrap();
        let r = s.status(t("2026-12-20T09:00:00Z")).await.unwrap();
        assert!(r.status.read_only());
    }

    #[tokio::test]
    async fn clock_rollback_is_remembered_in_the_database() {
        let s = svc("m").await;
        s.status(t("2026-12-01T09:00:00Z")).await.unwrap();
        s.status(t("2026-12-20T09:00:00Z")).await.unwrap();
        let r = s.status(t("2026-12-02T09:00:00Z")).await.unwrap();
        assert!(r.status.read_only());
    }

    #[tokio::test]
    async fn trial_token_is_fetched_verified_and_kept() {
        let s = svc("m").await;
        s.status(t("2026-12-10T09:00:00Z")).await.unwrap();
        assert!(s.fetch_trial_token().await);
        // The server saw this computer on Dec 1, earlier than the local start.
        let r = s.status(t("2026-12-20T09:00:00Z")).await.unwrap();
        assert!(r.status.read_only());
    }

    #[tokio::test]
    async fn offline_trial_token_fetch_is_quietly_false() {
        let s = svc("m").await;
        *s.api.fail.lock().unwrap() = Some(|| LicenseError::Network);
        assert!(!s.fetch_trial_token().await);
    }

    #[tokio::test]
    async fn activation_unlocks_and_survives_restart() {
        let s = svc("m").await;
        s.status(t("2026-12-01T09:00:00Z")).await.unwrap();
        let r = s
            .activate("ceev 7kq2m 9xb4d tr5vw h3nz8", t("2026-12-20T09:00:00Z"))
            .await
            .unwrap();
        assert_eq!(
            r.status,
            Status::Licensed {
                key: KEY.into(),
                seats: 2
            }
        );
        let r = s.status(t("2027-06-01T09:00:00Z")).await.unwrap();
        assert!(matches!(r.status, Status::Licensed { .. }));
    }

    #[tokio::test]
    async fn activation_errors_are_clear_and_change_nothing() {
        let s = svc("m").await;
        s.status(t("2026-12-01T09:00:00Z")).await.unwrap();
        assert!(matches!(
            s.activate("nope", t("2026-12-02T00:00:00Z")).await,
            Err(LicenseError::BadKeyFormat)
        ));
        *s.api.fail.lock().unwrap() = Some(|| LicenseError::SeatsFull);
        assert!(matches!(
            s.activate(KEY, t("2026-12-02T00:00:00Z")).await,
            Err(LicenseError::SeatsFull)
        ));
        *s.api.fail.lock().unwrap() = Some(|| LicenseError::Network);
        assert!(matches!(
            s.activate(KEY, t("2026-12-02T00:00:00Z")).await,
            Err(LicenseError::Network)
        ));
        let r = s.status(t("2026-12-02T00:00:00Z")).await.unwrap();
        assert!(matches!(r.status, Status::Trial { .. }));
    }

    #[tokio::test]
    async fn a_license_for_another_machine_or_version_is_refused() {
        let (sk, v) = keypair();
        let mk = |api_machine: &str, major: u32| {
            let mut api = FakeApi::new(sk.clone(), api_machine);
            api.major = major;
            api
        };
        for (api, want_wrong_machine) in [(mk("someone-else", 1), true), (mk("m", 0), false)] {
            let s = LicenseService::new(
                Db::open_in_memory().await.unwrap(),
                api,
                MemoryAnchor::default(),
                Some(v.clone()),
                "m".into(),
                "1.0.0",
            );
            let err = s
                .activate(KEY, t("2026-12-02T00:00:00Z"))
                .await
                .unwrap_err();
            if want_wrong_machine {
                assert!(matches!(err, LicenseError::WrongMachine));
            } else {
                assert!(matches!(err, LicenseError::WrongVersion));
            }
        }
    }

    #[tokio::test]
    async fn no_public_key_means_activation_is_unavailable() {
        let (sk, _) = keypair();
        let s = LicenseService::new(
            Db::open_in_memory().await.unwrap(),
            FakeApi::new(sk, "m"),
            MemoryAnchor::default(),
            None,
            "m".into(),
            "1.0.0",
        );
        assert!(matches!(
            s.activate(KEY, t("2026-12-02T00:00:00Z")).await,
            Err(LicenseError::NoPublicKey)
        ));
    }

    #[tokio::test]
    async fn deactivation_frees_the_seat_then_returns_to_the_trial_state() {
        let s = svc("m").await;
        s.status(t("2026-12-01T09:00:00Z")).await.unwrap();
        s.activate(KEY, t("2026-12-02T09:00:00Z")).await.unwrap();
        // Server unreachable: the license stays.
        *s.api.fail.lock().unwrap() = Some(|| LicenseError::Network);
        assert!(s.deactivate(t("2026-12-30T09:00:00Z")).await.is_err());
        assert!(matches!(
            s.status(t("2026-12-30T09:00:00Z")).await.unwrap().status,
            Status::Licensed { .. }
        ));
        *s.api.fail.lock().unwrap() = None;
        let r = s.deactivate(t("2026-12-30T09:00:00Z")).await.unwrap();
        assert!(r.status.read_only(), "trial is long over");
        assert_eq!(s.api.deactivated.lock().unwrap().as_slice(), [KEY]);
    }
}
