//! strftime-subset formatting of a [`DateTime`].
//!
//! Pattern: Interpreter — walks the format string and renders one directive at a time.

use std::fmt::Write as _;

use super::datetime::DateTime;
use super::error::TemplateError;

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
const DAYS: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];

impl DateTime {
    /// Format with `%Y %y %m %d %e %H %M %S %j %B %b %A %a %%`.
    ///
    /// # Errors
    /// [`TemplateError::BadFormat`] for an unknown directive.
    pub fn format(&self, fmt: &str) -> Result<String, TemplateError> {
        let mut out = String::new();
        let mut it = fmt.chars();
        while let Some(c) = it.next() {
            if c != '%' {
                out.push(c);
                continue;
            }
            match it.next() {
                Some('Y') => { let _ = write!(out, "{:04}", self.year); }
                Some('y') => { let _ = write!(out, "{:02}", self.year.rem_euclid(100)); }
                Some('m') => { let _ = write!(out, "{:02}", self.month); }
                Some('d') => { let _ = write!(out, "{:02}", self.day); }
                Some('e') => { let _ = write!(out, "{:2}", self.day); }
                Some('H') => { let _ = write!(out, "{:02}", self.hour); }
                Some('M') => { let _ = write!(out, "{:02}", self.minute); }
                Some('S') => { let _ = write!(out, "{:02}", self.second); }
                Some('j') => { let _ = write!(out, "{:03}", self.yday); }
                Some('B') => out.push_str(MONTHS[(self.month - 1) as usize]),
                Some('b') => out.push_str(&MONTHS[(self.month - 1) as usize][..3]),
                Some('A') => out.push_str(DAYS[self.weekday as usize]),
                Some('a') => out.push_str(&DAYS[self.weekday as usize][..3]),
                Some('%') | None => out.push('%'),
                Some(o) => return Err(TemplateError::BadFormat(o)),
            }
        }
        Ok(out)
    }
}
