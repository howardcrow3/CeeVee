//! What the machine can run.

/// Total physical memory in whole gigabytes (rounded to nearest, so a
/// "16 GB" laptop that reports 15.6 GiB counts as 16).
pub fn total_ram_gb() -> u64 {
    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    bytes_to_gb(sys.total_memory())
}

pub fn bytes_to_gb(bytes: u64) -> u64 {
    const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
    (bytes as f64 / GIB).round() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounds_to_nearest_gigabyte() {
        assert_eq!(bytes_to_gb(16 * 1024 * 1024 * 1024), 16);
        assert_eq!(bytes_to_gb(15_600_000_000), 15); // 14.53 GiB
        assert_eq!(bytes_to_gb(17_000_000_000), 16);
        assert_eq!(bytes_to_gb(0), 0);
    }

    #[test]
    fn this_machine_reports_some_memory() {
        assert!(total_ram_gb() >= 1);
    }
}
