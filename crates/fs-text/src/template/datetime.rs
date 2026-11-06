//! Calendar date + time (UTC) without external crates.
//!
//! Pattern: Value object — immutable, `Copy`, built from Unix seconds (Hinnant's civil-date
//! algorithm).

/// Calendar date + time (UTC) without external crates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DateTime {
    /// Year.
    pub year: i64,
    /// Month 1..=12.
    pub month: u32,
    /// Day of month.
    pub day: u32,
    /// Hour.
    pub hour: u32,
    /// Minute.
    pub minute: u32,
    /// Second.
    pub second: u32,
    /// 0 = Monday … 6 = Sunday
    pub weekday: u32,
    /// 1-based day of year
    pub yday: u32,
}

impl DateTime {
    /// Convert Unix seconds (UTC).
    #[must_use]
    pub fn from_unix(secs: i64) -> Self {
        let days = secs.div_euclid(86_400);
        let rem = secs.rem_euclid(86_400) as u32;
        // civil from days (Howard Hinnant)
        let z = days + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z.rem_euclid(146_097);
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
        let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
        let year = if m <= 2 { y + 1 } else { y };
        let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
        let cum = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
        let yday = cum[(m - 1) as usize] + d + if leap && m > 2 { 1 } else { 0 };
        Self {
            year,
            month: m,
            day: d,
            hour: rem / 3600,
            minute: rem % 3600 / 60,
            second: rem % 60,
            weekday: (days + 3).rem_euclid(7) as u32,
            yday,
        }
    }
    /// The current time.
    #[must_use]
    pub fn now() -> Self {
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(0));
        Self::from_unix(secs)
    }
}
