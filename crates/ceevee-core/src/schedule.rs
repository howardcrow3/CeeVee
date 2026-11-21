//! Pure scheduling rules (docs/TECHNICAL_PLAN.md, section 3.1).
//!
//! Nothing here touches the clock or the disk: callers pass `now` and the last
//! run times, so every rule is unit-testable. A run missed while the computer
//! slept or was off fires once at the next check, never once per missed day.

use chrono::{DateTime, Duration, Local, NaiveTime, TimeZone};

/// When a task should run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Schedule {
    /// Once a day at a local wall-clock time (for example 7:00).
    Daily { hour: u32, minute: u32 },
    /// At a fixed interval after the previous run.
    Every(Duration),
}

/// A registered recurring task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    pub id: String,
    pub schedule: Schedule,
}

/// Conditions under which background work must not start.
#[derive(Debug, Clone, Default)]
pub struct Gate {
    /// "Pause background work" from the tray menu.
    pub paused_until: Option<DateTime<Local>>,
    /// Quiet hours as local start and end; may wrap midnight (22:00 to 07:00).
    pub quiet_hours: Option<(NaiveTime, NaiveTime)>,
    /// Battery percentage when running on battery; `None` on mains power or unknown.
    pub battery_percent: Option<u8>,
}

/// Background work pauses below this battery level.
pub const LOW_BATTERY_PERCENT: u8 = 20;

impl Gate {
    pub fn is_open(&self, now: DateTime<Local>) -> bool {
        if self.paused_until.is_some_and(|until| now < until) {
            return false;
        }
        if let Some((start, end)) = self.quiet_hours {
            let t = now.time();
            let quiet = if start <= end {
                t >= start && t < end
            } else {
                t >= start || t < end
            };
            if quiet {
                return false;
            }
        }
        !self
            .battery_percent
            .is_some_and(|p| p < LOW_BATTERY_PERCENT)
    }
}

/// The most recent scheduled instant of `schedule` at or before `now`, for `Daily`.
fn latest_daily_occurrence(
    now: DateTime<Local>,
    hour: u32,
    minute: u32,
) -> Option<DateTime<Local>> {
    let at = |date: chrono::NaiveDate| {
        // `earliest` resolves a daylight-saving fold; a spring-forward gap yields None.
        Local
            .from_local_datetime(&date.and_hms_opt(hour, minute, 0)?)
            .earliest()
    };
    let today = now.date_naive();
    match at(today) {
        Some(t) if t <= now => Some(t),
        _ => at(today.pred_opt()?),
    }
}

/// Whether `task` is due at `now`, given when it last ran (`None` = never).
pub fn is_due(task: &Task, last_run: Option<DateTime<Local>>, now: DateTime<Local>) -> bool {
    match task.schedule {
        Schedule::Every(every) => last_run.is_none_or(|last| now - last >= every),
        Schedule::Daily { hour, minute } => {
            let Some(latest) = latest_daily_occurrence(now, hour, minute) else {
                return false;
            };
            // A fresh install does not run yesterday's slot, only the next one.
            last_run.is_some_and(|last| last < latest)
        }
    }
}

/// Ids of the tasks that should start now. Empty while the gate is closed.
pub fn due_tasks<'a>(
    tasks: &'a [Task],
    last_runs: impl Fn(&str) -> Option<DateTime<Local>>,
    gate: &Gate,
    now: DateTime<Local>,
) -> Vec<&'a str> {
    if !gate.is_open(now) {
        return Vec::new();
    }
    tasks
        .iter()
        .filter(|t| is_due(t, last_runs(&t.id), now))
        .map(|t| t.id.as_str())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(y: i32, mo: u32, d: u32, h: u32, mi: u32) -> DateTime<Local> {
        Local
            .with_ymd_and_hms(y, mo, d, h, mi, 0)
            .earliest()
            .unwrap()
    }
    fn daily() -> Task {
        Task {
            id: "search".into(),
            schedule: Schedule::Daily { hour: 7, minute: 0 },
        }
    }

    #[test]
    fn daily_runs_after_its_time_once() {
        let last = Some(at(2026, 11, 10, 7, 0));
        assert!(!is_due(&daily(), last, at(2026, 11, 11, 6, 59)));
        assert!(is_due(&daily(), last, at(2026, 11, 11, 7, 0)));
        let ran = Some(at(2026, 11, 11, 7, 1));
        assert!(!is_due(&daily(), ran, at(2026, 11, 11, 20, 0)));
    }

    #[test]
    fn several_missed_days_run_once() {
        let last = Some(at(2026, 11, 5, 7, 0));
        let now = at(2026, 11, 11, 9, 0);
        assert!(is_due(&daily(), last, now));
        // After that single run, it is quiet until tomorrow.
        assert!(!is_due(&daily(), Some(now), at(2026, 11, 11, 23, 0)));
        assert!(is_due(&daily(), Some(now), at(2026, 11, 12, 7, 0)));
    }

    #[test]
    fn fresh_install_waits_for_the_next_slot() {
        assert!(!is_due(&daily(), None, at(2026, 11, 11, 9, 0)));
    }

    #[test]
    fn interval_tasks_run_when_never_run_or_elapsed() {
        let t = Task {
            id: "mail".into(),
            schedule: Schedule::Every(Duration::minutes(10)),
        };
        let now = at(2026, 11, 11, 12, 0);
        assert!(is_due(&t, None, now));
        assert!(!is_due(&t, Some(now - Duration::minutes(9)), now));
        assert!(is_due(&t, Some(now - Duration::minutes(10)), now));
    }

    #[test]
    fn gate_blocks_pause_quiet_hours_and_low_battery() {
        let now = at(2026, 11, 11, 23, 30);
        assert!(Gate::default().is_open(now));
        let paused = Gate {
            paused_until: Some(now + Duration::hours(1)),
            ..Gate::default()
        };
        assert!(!paused.is_open(now));
        assert!(paused.is_open(now + Duration::hours(2)));
        let t = |h, m| NaiveTime::from_hms_opt(h, m, 0).unwrap();
        let quiet = Gate {
            quiet_hours: Some((t(22, 0), t(7, 0))),
            ..Gate::default()
        };
        assert!(!quiet.is_open(now));
        assert!(!quiet.is_open(at(2026, 11, 12, 6, 59)));
        assert!(quiet.is_open(at(2026, 11, 12, 7, 0)));
        let low = Gate {
            battery_percent: Some(19),
            ..Gate::default()
        };
        assert!(!low.is_open(now));
        assert!(Gate {
            battery_percent: Some(20),
            ..Gate::default()
        }
        .is_open(now));
    }

    #[test]
    fn due_tasks_respects_the_gate() {
        let tasks = vec![
            daily(),
            Task {
                id: "mail".into(),
                schedule: Schedule::Every(Duration::minutes(10)),
            },
        ];
        let now = at(2026, 11, 11, 9, 0);
        let last = |id: &str| (id == "search").then(|| at(2026, 11, 10, 7, 0));
        assert_eq!(
            due_tasks(&tasks, last, &Gate::default(), now),
            vec!["search", "mail"]
        );
        let closed = Gate {
            battery_percent: Some(5),
            ..Gate::default()
        };
        assert!(due_tasks(&tasks, last, &closed, now).is_empty());
    }
}
