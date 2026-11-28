//! One scheduling pass. Rules live in `ceevee_core::schedule`; this module reads
//! settings, asks the rules what is due, and publishes `TaskDue`.
//!
//! Comparing the wall clock on every tick is also how sleep and wake are
//! handled: the first tick after the computer wakes sees the missed slot and
//! fires it once.

use ceevee_core::schedule::{due_tasks, Gate, Schedule, Task};
use ceevee_core::{Event, EventBus};
use ceevee_db::Db;
use chrono::{DateTime, Duration, Local, NaiveTime};

const LAST_RUN_PREFIX: &str = "schedule.last_run.";

/// Default recurring tasks (TECHNICAL_PLAN 3.1). Feature crates subscribe to
/// `TaskDue` for the ids they own.
pub fn default_tasks() -> Vec<Task> {
    vec![
        Task {
            id: "search.daily".into(),
            schedule: Schedule::Daily { hour: 7, minute: 0 },
        },
        Task {
            id: "mail.sync".into(),
            schedule: Schedule::Every(Duration::minutes(10)),
        },
        Task {
            id: "followups.daily".into(),
            schedule: Schedule::Daily { hour: 9, minute: 0 },
        },
    ]
}

fn parse_time(s: &str) -> Option<NaiveTime> {
    NaiveTime::parse_from_str(s, "%H:%M").ok()
}

async fn load_gate(db: &Db) -> Gate {
    let paused_until = db
        .get_setting("background.paused_until")
        .await
        .ok()
        .flatten()
        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
        .map(|d| d.with_timezone(&Local));
    let start = db
        .get_setting("background.quiet_start")
        .await
        .ok()
        .flatten();
    let end = db.get_setting("background.quiet_end").await.ok().flatten();
    let quiet_hours = start
        .as_deref()
        .and_then(parse_time)
        .zip(end.as_deref().and_then(parse_time));
    // The platform call can block briefly, so keep it off the async threads.
    let battery_percent = tokio::task::spawn_blocking(crate::power::battery_percent)
        .await
        .ok()
        .flatten();
    Gate {
        paused_until,
        quiet_hours,
        battery_percent,
    }
}

async fn last_run(db: &Db, id: &str) -> Option<DateTime<Local>> {
    let raw = db
        .get_setting(&format!("{LAST_RUN_PREFIX}{id}"))
        .await
        .ok()
        .flatten()?;
    DateTime::parse_from_rfc3339(&raw)
        .ok()
        .map(|d| d.with_timezone(&Local))
}

async fn record_run(db: &Db, id: &str, at: DateTime<Local>) {
    if let Err(e) = db
        .set_setting(&format!("{LAST_RUN_PREFIX}{id}"), &at.to_rfc3339())
        .await
    {
        tracing::warn!("could not record run of {id}: {e}");
    }
}

/// Run one scheduling pass. Returns the ids that were published as due.
pub async fn tick(db: &Db, bus: &EventBus, tasks: &[Task], now: DateTime<Local>) -> Vec<String> {
    // Seed tasks that have never been seen so a fresh install waits for its next slot.
    let mut last = std::collections::HashMap::new();
    for t in tasks {
        match last_run(db, &t.id).await {
            Some(at) => {
                last.insert(t.id.clone(), at);
            }
            None if matches!(t.schedule, Schedule::Daily { .. }) => {
                record_run(db, &t.id, now).await;
                last.insert(t.id.clone(), now);
            }
            None => {}
        }
    }
    let gate = load_gate(db).await;
    let due: Vec<String> = due_tasks(tasks, |id| last.get(id).copied(), &gate, now)
        .into_iter()
        .map(str::to_owned)
        .collect();
    for id in &due {
        record_run(db, id, now).await;
        bus.publish(Event::TaskDue { task: id.clone() });
    }
    due
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(d: u32, h: u32, m: u32) -> DateTime<Local> {
        Local
            .with_ymd_and_hms(2026, 11, d, h, m, 0)
            .earliest()
            .unwrap()
    }

    #[tokio::test]
    async fn fresh_install_then_wake_after_days_fires_once() {
        let db = Db::open_in_memory().await.unwrap();
        let bus = EventBus::new(16);
        let mut rx = bus.subscribe();
        let tasks = vec![Task {
            id: "search.daily".into(),
            schedule: Schedule::Daily { hour: 7, minute: 0 },
        }];

        assert!(
            tick(&db, &bus, &tasks, at(1, 12, 0)).await.is_empty(),
            "first sight only seeds"
        );
        // The computer sleeps for ten days, then wakes at 9:00.
        assert_eq!(
            tick(&db, &bus, &tasks, at(11, 9, 0)).await,
            vec!["search.daily"]
        );
        assert!(
            tick(&db, &bus, &tasks, at(11, 9, 1)).await.is_empty(),
            "only once"
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            Event::TaskDue {
                task: "search.daily".into()
            }
        );
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn pause_setting_holds_work_back() {
        let db = Db::open_in_memory().await.unwrap();
        let bus = EventBus::new(16);
        let tasks = vec![Task {
            id: "mail.sync".into(),
            schedule: Schedule::Every(Duration::minutes(10)),
        }];
        let now = at(11, 12, 0);
        db.set_setting(
            "background.paused_until",
            &(now + Duration::hours(1)).to_rfc3339(),
        )
        .await
        .unwrap();
        assert!(tick(&db, &bus, &tasks, now).await.is_empty());
        assert_eq!(
            tick(&db, &bus, &tasks, now + Duration::hours(2)).await,
            vec!["mail.sync"]
        );
    }
}
