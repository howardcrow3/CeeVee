//! The rules, with no I/O: stored facts in, [`Status`] out.

use chrono::{DateTime, Duration, Utc};

use crate::token::{Claims, Verifier};

pub const TRIAL_DAYS: i64 = 14;

/// Where the person stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    /// No start recorded yet (the service records it on first run).
    NotStarted,
    Trial {
        ends_at: DateTime<Utc>,
        days_left: u32,
    },
    /// Trial over, no license: the app is read-only (view and export).
    Expired {
        ended_at: DateTime<Utc>,
    },
    Licensed {
        key: String,
        seats: u32,
    },
}

impl Status {
    /// Read-only means view and export only.
    pub fn read_only(&self) -> bool {
        matches!(self, Status::Expired { .. })
    }
}

/// What is stored on this computer.
#[derive(Debug, Clone, Default)]
pub struct Facts {
    pub db_trial_start: Option<DateTime<Utc>>,
    pub anchor_trial_start: Option<DateTime<Utc>>,
    pub trial_token: Option<String>,
    pub license_blob: Option<String>,
    /// The newest time ever seen by the app (clock rollback guard).
    pub last_seen: Option<DateTime<Utc>>,
}

/// Why a stored license was ignored, if it was (shown in Settings).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    BadSignature,
    WrongMachine,
    WrongVersion,
}

pub struct Evaluation {
    pub status: Status,
    pub problem: Option<Problem>,
    /// The trial start the rules settled on (earliest of every source), if any.
    pub trial_start: Option<DateTime<Utc>>,
}

/// The time to trust: the clock, but never earlier than the newest time seen.
pub fn effective_now(now: DateTime<Utc>, last_seen: Option<DateTime<Utc>>) -> DateTime<Utc> {
    last_seen.map_or(now, |l| l.max(now))
}

/// Decide the status. `verifier` is `None` in a build with no public key, where
/// every license and token fails closed.
pub fn evaluate(
    facts: &Facts,
    now: DateTime<Utc>,
    machine: &str,
    app_major: u32,
    verifier: Option<&Verifier>,
) -> Evaluation {
    let now = effective_now(now, facts.last_seen);

    let mut problem = None;
    if let Some(blob) = facts
        .license_blob
        .as_deref()
        .filter(|b| !b.trim().is_empty())
    {
        match verifier.map(|v| v.verify(blob)) {
            Some(Ok(Claims::License {
                key,
                machine: m,
                major,
                seats,
                ..
            })) => {
                if m != machine {
                    problem = Some(Problem::WrongMachine);
                } else if major < app_major {
                    problem = Some(Problem::WrongVersion);
                } else {
                    return Evaluation {
                        status: Status::Licensed { key, seats },
                        problem: None,
                        trial_start: trial_start(facts, machine, verifier),
                    };
                }
            }
            _ => problem = Some(Problem::BadSignature),
        }
    }

    let start = trial_start(facts, machine, verifier);
    let status = match start {
        None => Status::NotStarted,
        Some(start) => {
            let ends_at = start + Duration::days(TRIAL_DAYS);
            if now >= ends_at {
                Status::Expired { ended_at: ends_at }
            } else {
                let left = ends_at - now;
                // Round up: with 3 hours left it is still "1 day left".
                let days_left = ((left.num_seconds() + 86_399) / 86_400) as u32;
                Status::Trial { ends_at, days_left }
            }
        }
    };
    Evaluation {
        status,
        problem,
        trial_start: start,
    }
}

/// Earliest start among the database, the keychain and a verified server token.
/// Earliest wins so wiping one place does not restart the trial.
fn trial_start(facts: &Facts, machine: &str, verifier: Option<&Verifier>) -> Option<DateTime<Utc>> {
    let token = facts
        .trial_token
        .as_deref()
        .and_then(|t| match verifier?.verify(t) {
            Ok(Claims::Trial {
                machine: m,
                started_at,
            }) if m == machine => Some(started_at),
            _ => None,
        });
    [facts.db_trial_start, facts.anchor_trial_start, token]
        .into_iter()
        .flatten()
        .min()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::{sign, testing::keypair};

    fn t(s: &str) -> DateTime<Utc> {
        s.parse().unwrap()
    }

    fn facts(start: &str) -> Facts {
        Facts {
            db_trial_start: Some(t(start)),
            ..Default::default()
        }
    }

    fn eval(f: &Facts, now: &str) -> Status {
        evaluate(f, t(now), "m", 1, None).status
    }

    #[test]
    fn nothing_stored_is_not_started() {
        assert_eq!(
            eval(&Facts::default(), "2026-12-01T00:00:00Z"),
            Status::NotStarted
        );
    }

    #[test]
    fn trial_counts_down_then_expires_at_exactly_14_days() {
        let f = facts("2026-12-01T00:00:00Z");
        assert!(matches!(
            eval(&f, "2026-12-01T00:00:00Z"),
            Status::Trial { days_left: 14, .. }
        ));
        assert!(matches!(
            eval(&f, "2026-12-01T10:00:00Z"),
            Status::Trial { days_left: 14, .. }
        ));
        assert!(matches!(
            eval(&f, "2026-12-14T21:00:00Z"),
            Status::Trial { days_left: 1, .. }
        ));
        assert!(matches!(
            eval(&f, "2026-12-14T23:59:59Z"),
            Status::Trial { days_left: 1, .. }
        ));
        let s = eval(&f, "2026-12-15T00:00:00Z");
        assert_eq!(
            s,
            Status::Expired {
                ended_at: t("2026-12-15T00:00:00Z")
            }
        );
        assert!(s.read_only());
    }

    #[test]
    fn earliest_start_wins_so_wiping_one_place_does_not_reset() {
        let f = Facts {
            db_trial_start: Some(t("2026-12-10T00:00:00Z")),
            anchor_trial_start: Some(t("2026-12-01T00:00:00Z")),
            ..Default::default()
        };
        assert!(matches!(
            eval(&f, "2026-12-20T00:00:00Z"),
            Status::Expired { .. }
        ));
        // Keychain wiped, database kept.
        let f = facts("2026-12-01T00:00:00Z");
        assert!(matches!(
            eval(&f, "2026-12-20T00:00:00Z"),
            Status::Expired { .. }
        ));
    }

    #[test]
    fn rolling_the_clock_back_does_not_extend_the_trial() {
        let mut f = facts("2026-12-01T00:00:00Z");
        f.last_seen = Some(t("2026-12-20T00:00:00Z"));
        assert!(matches!(
            eval(&f, "2026-12-02T00:00:00Z"),
            Status::Expired { .. }
        ));
    }

    #[test]
    fn verified_server_token_can_only_shorten_the_trial() {
        let (sk, v) = keypair();
        let mut f = facts("2026-12-10T00:00:00Z");
        f.trial_token = Some(sign(
            &sk,
            &Claims::Trial {
                machine: "m".into(),
                started_at: t("2026-12-01T00:00:00Z"),
            },
        ));
        let e = evaluate(&f, t("2026-12-20T00:00:00Z"), "m", 1, Some(&v));
        assert!(matches!(e.status, Status::Expired { .. }));
        // A token for another machine, or with no key to check it, is ignored.
        f.trial_token = Some(sign(
            &sk,
            &Claims::Trial {
                machine: "other".into(),
                started_at: t("2026-12-01T00:00:00Z"),
            },
        ));
        let e = evaluate(&f, t("2026-12-20T00:00:00Z"), "m", 1, Some(&v));
        assert!(matches!(e.status, Status::Trial { .. }));
        let e = evaluate(&f, t("2026-12-20T00:00:00Z"), "m", 1, None);
        assert!(matches!(e.status, Status::Trial { .. }));
    }

    fn lic(machine: &str, major: u32) -> Claims {
        Claims::License {
            key: "CEEV-AAAAA-BBBBB-CCCCC-DDDDD".into(),
            machine: machine.into(),
            issued_at: t("2026-12-05T00:00:00Z"),
            major,
            seats: 2,
        }
    }

    #[test]
    fn a_valid_license_unlocks_after_the_trial() {
        let (sk, v) = keypair();
        let mut f = facts("2026-12-01T00:00:00Z");
        f.license_blob = Some(sign(&sk, &lic("m", 1)));
        let e = evaluate(&f, t("2027-03-01T00:00:00Z"), "m", 1, Some(&v));
        assert_eq!(
            e.status,
            Status::Licensed {
                key: "CEEV-AAAAA-BBBBB-CCCCC-DDDDD".into(),
                seats: 2
            }
        );
        assert_eq!(e.problem, None);
        assert!(!e.status.read_only());
    }

    #[test]
    fn bad_licenses_are_ignored_and_explained() {
        let (sk, v) = keypair();
        let (other_sk, _) = keypair();
        let cases = [
            (sign(&sk, &lic("other-machine", 1)), Problem::WrongMachine),
            (
                sign(&sk, &lic("m", 1)).replace('.', "x."),
                Problem::BadSignature,
            ),
            (sign(&other_sk, &lic("m", 1)), Problem::BadSignature),
            (
                sign(
                    &sk,
                    &Claims::Trial {
                        machine: "m".into(),
                        started_at: t("2026-12-01T00:00:00Z"),
                    },
                ),
                Problem::BadSignature,
            ),
        ];
        for (blob, want) in cases {
            let mut f = facts("2026-12-01T00:00:00Z");
            f.license_blob = Some(blob);
            let e = evaluate(&f, t("2027-03-01T00:00:00Z"), "m", 1, Some(&v));
            assert!(matches!(e.status, Status::Expired { .. }));
            assert_eq!(e.problem, Some(want));
        }
        // A license for 1.x does not unlock 2.x.
        let mut f = facts("2026-12-01T00:00:00Z");
        f.license_blob = Some(sign(&sk, &lic("m", 1)));
        let e = evaluate(&f, t("2027-03-01T00:00:00Z"), "m", 2, Some(&v));
        assert_eq!(e.problem, Some(Problem::WrongVersion));
    }

    #[test]
    fn no_public_key_fails_closed() {
        let (sk, _) = keypair();
        let mut f = facts("2026-12-01T00:00:00Z");
        f.license_blob = Some(sign(&sk, &lic("m", 1)));
        let e = evaluate(&f, t("2027-03-01T00:00:00Z"), "m", 1, None);
        assert!(matches!(e.status, Status::Expired { .. }));
    }
}
