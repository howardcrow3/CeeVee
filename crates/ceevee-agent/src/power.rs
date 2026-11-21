//! Reading the battery level for the scheduler's low-battery rule.
//!
//! Only a battery that is actually discharging counts: on mains power, charging, full, or a
//! desktop with no battery, there is nothing to protect and the answer is `None`.

use starship_battery::units::ratio::percent;
use starship_battery::{Manager, State};

/// What one battery reported.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reading {
    pub discharging: bool,
    /// 0.0 to 100.0.
    pub percent: f32,
}

/// The lowest percentage among discharging batteries, rounded down; `None` if none is.
pub fn low_water_mark(readings: &[Reading]) -> Option<u8> {
    readings
        .iter()
        .filter(|r| r.discharging)
        .map(|r| r.percent.clamp(0.0, 100.0))
        .reduce(f32::min)
        .map(|p| p.floor() as u8)
}

/// Read the system's batteries. Any error (no battery, no permission) means "unknown", which the
/// scheduler treats as "plenty".
pub fn battery_percent() -> Option<u8> {
    let manager = Manager::new().ok()?;
    let readings: Vec<Reading> = manager
        .batteries()
        .ok()?
        .filter_map(Result::ok)
        .map(|b| Reading {
            discharging: b.state() == State::Discharging,
            percent: b.state_of_charge().get::<percent>(),
        })
        .collect();
    low_water_mark(&readings)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(discharging: bool, level: f32) -> Reading {
        Reading {
            discharging,
            percent: level,
        }
    }

    #[test]
    fn no_batteries_or_mains_power_means_none() {
        assert_eq!(low_water_mark(&[]), None);
        assert_eq!(low_water_mark(&[r(false, 15.0)]), None);
    }

    #[test]
    fn a_discharging_battery_reports_its_level_rounded_down() {
        assert_eq!(low_water_mark(&[r(true, 19.9)]), Some(19));
        assert_eq!(low_water_mark(&[r(true, 100.0)]), Some(100));
    }

    #[test]
    fn the_lowest_discharging_battery_wins_and_others_are_ignored() {
        assert_eq!(
            low_water_mark(&[r(true, 60.0), r(true, 12.0), r(false, 3.0)]),
            Some(12)
        );
    }

    #[test]
    fn out_of_range_values_are_clamped() {
        assert_eq!(low_water_mark(&[r(true, -4.0)]), Some(0));
        assert_eq!(low_water_mark(&[r(true, 140.0)]), Some(100));
    }

    #[test]
    fn reading_this_machine_never_panics() {
        let _ = battery_percent();
    }
}
