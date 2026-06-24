//! Shared timestamp formatting utilities for the scryrs server crate.
//!
//! Extracted from `store.rs` and `server.rs` to avoid duplicated
//! `chrono_now` / `civil_from_days` logic (AGENTS.md Rule 8).

/// Return the current wall-clock time as an RFC 3339 string.
///
/// We avoid a chrono dependency by formatting manually from
/// [`std::time::SystemTime`]. The output is always UTC (suffix `Z`).
pub(crate) fn chrono_now() -> String {
    use std::time::SystemTime;
    let dur = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();
    let days_since_epoch = secs / 86400;
    let (year, month, day) = civil_from_days(days_since_epoch as i64);
    let remaining = secs % 86400;
    let hours = remaining / 3600;
    let minutes = (remaining % 3600) / 60;
    let seconds = remaining % 60;
    format!("{year:04}-{month:02}-{day:02}T{hours:02}:{minutes:02}:{seconds:02}Z")
}

/// Simplified civil date from days since Unix epoch (1970-01-01).
///
/// Accurate enough for current timestamps; not a full calendar
/// implementation.
pub(crate) fn civil_from_days(mut days: i64) -> (i64, u32, u32) {
    days += 719468; // shift from Unix epoch to 0000-03-01 (start of Gregorian cycle)
    let era = if days >= 0 {
        days / 146097
    } else {
        (days - 146096) / 146097
    };
    let doe = days - era * 146097; // day of era [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // year of era [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // day of year [0, 365]
    let mp = (5 * doy + 2) / 153; // month phase [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // day of month [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m as u32, d as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_from_days_known_epoch() {
        // 1970-01-01 is day 0 from Unix epoch.
        let (y, m, d) = civil_from_days(0);
        assert_eq!(y, 1970);
        assert_eq!(m, 1);
        assert_eq!(d, 1);
    }

    #[test]
    fn civil_from_days_2026_06_24() {
        // 2026-06-24 is 20628 days after epoch.
        let (y, m, d) = civil_from_days(20628);
        assert_eq!(y, 2026);
        assert_eq!(m, 6);
        assert_eq!(d, 24);
    }

    #[test]
    fn chrono_now_produces_rfc3339_shape() {
        let ts = chrono_now();
        // Must have YYYY-MM-DDTHH:MM:SSZ shape.
        assert!(ts.len() >= 20);
        assert!(ts.ends_with('Z'));
        let bytes = ts.as_bytes();
        assert_eq!(bytes[4], b'-');
        assert_eq!(bytes[7], b'-');
        assert_eq!(bytes[10], b'T');
        assert_eq!(bytes[13], b':');
        assert_eq!(bytes[16], b':');
    }
}
