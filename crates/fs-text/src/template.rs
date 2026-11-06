//! Templates (ADR 0042 §9): `{{name}}` and `{{date:FORMAT}}` placeholders, plus
//! variable point marks (ADR 0033) in block documents.
//!
//! `\{{` writes a literal `{{`. Date formats use a strftime subset:
//! `%Y %y %m %d %e %H %M %S %B %b %A %a %j %%`.

use crate::doc::Document;
use crate::id::{new_id, Uuid};
use crate::mark::*;
use std::collections::BTreeMap;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum TemplateError {
    #[error("no value for variable `{0}`")]
    Missing(String),
    #[error("unknown date format directive `%{0}`")]
    BadFormat(char),
}

/// Calendar date + time (UTC) without external crates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DateTime {
    pub year: i64,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
    /// 0 = Monday … 6 = Sunday
    pub weekday: u32,
    /// 1-based day of year
    pub yday: u32,
}

const MONTHS: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
const DAYS: [&str; 7] = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];

impl DateTime {
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
        Self { year, month: m, day: d, hour: rem / 3600, minute: rem % 3600 / 60, second: rem % 60, weekday: (days + 3).rem_euclid(7) as u32, yday }
    }
    pub fn now() -> Self {
        let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
        Self::from_unix(secs)
    }
    pub fn format(&self, fmt: &str) -> Result<String, TemplateError> {
        let mut out = String::new();
        let mut it = fmt.chars();
        while let Some(c) = it.next() {
            if c != '%' {
                out.push(c);
                continue;
            }
            match it.next() {
                Some('Y') => out.push_str(&format!("{:04}", self.year)),
                Some('y') => out.push_str(&format!("{:02}", self.year.rem_euclid(100))),
                Some('m') => out.push_str(&format!("{:02}", self.month)),
                Some('d') => out.push_str(&format!("{:02}", self.day)),
                Some('e') => out.push_str(&format!("{:2}", self.day)),
                Some('H') => out.push_str(&format!("{:02}", self.hour)),
                Some('M') => out.push_str(&format!("{:02}", self.minute)),
                Some('S') => out.push_str(&format!("{:02}", self.second)),
                Some('j') => out.push_str(&format!("{:03}", self.yday)),
                Some('B') => out.push_str(MONTHS[(self.month - 1) as usize]),
                Some('b') => out.push_str(&MONTHS[(self.month - 1) as usize][..3]),
                Some('A') => out.push_str(DAYS[self.weekday as usize]),
                Some('a') => out.push_str(&DAYS[self.weekday as usize][..3]),
                Some('%') => out.push('%'),
                Some(o) => return Err(TemplateError::BadFormat(o)),
                None => out.push('%'),
            }
        }
        Ok(out)
    }
}

/// Values for instantiation. Entered values go into the database *and* the document.
#[derive(Clone, Debug)]
pub struct Values {
    pub vars: BTreeMap<String, String>,
    pub now: DateTime,
    /// Leave unknown placeholders in place instead of failing.
    pub lenient: bool,
}

impl Values {
    pub fn new(now: DateTime) -> Self {
        Self { vars: BTreeMap::new(), now, lenient: false }
    }
    pub fn set(mut self, k: &str, v: &str) -> Self {
        self.vars.insert(k.to_string(), v.to_string());
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Placeholder {
    /// Char range including the braces.
    pub start: usize,
    pub end: usize,
    pub name: String,
    /// `Some(fmt)` for `{{date:fmt}}`.
    pub date_format: Option<String>,
}

/// Find placeholders in `text` (char offsets). `\{{` is not a placeholder.
pub fn find_placeholders(text: &str) -> Vec<Placeholder> {
    let chars: Vec<char> = text.chars().collect();
    let mut v = Vec::new();
    let mut i = 0;
    while i + 1 < chars.len() {
        if chars[i] == '{' && chars[i + 1] == '{' && !(i > 0 && chars[i - 1] == '\\') {
            if let Some(rel) = (i + 2..chars.len().saturating_sub(1)).find(|&j| chars[j] == '}' && chars[j + 1] == '}') {
                let inner: String = chars[i + 2..rel].iter().collect();
                let inner_t = inner.trim();
                let ok = !inner_t.is_empty() && !inner.contains('\n') && !inner.contains('{');
                if ok {
                    let (name, fmt) = match inner_t.split_once(':') {
                        Some(("date", f)) => ("date".to_string(), Some(f.to_string())),
                        _ => (inner_t.to_string(), None),
                    };
                    v.push(Placeholder { start: i, end: rel + 2, name, date_format: fmt });
                    i = rel + 2;
                    continue;
                }
            }
        }
        i += 1;
    }
    v
}

/// Names of all variables a template needs (`date` placeholders excluded).
pub fn variables(text: &str) -> Vec<String> {
    let mut names: Vec<String> = find_placeholders(text).into_iter().filter(|p| p.date_format.is_none()).map(|p| p.name).collect();
    names.sort();
    names.dedup();
    names
}

fn resolve(p: &Placeholder, vals: &Values) -> Result<Option<String>, TemplateError> {
    if let Some(f) = &p.date_format {
        return vals.now.format(f).map(Some);
    }
    match vals.vars.get(&p.name) {
        Some(v) => Ok(Some(v.clone())),
        None if vals.lenient => Ok(None),
        None => Err(TemplateError::Missing(p.name.clone())),
    }
}

/// Fill a plain-text template.
pub fn instantiate_str(template: &str, vals: &Values) -> Result<String, TemplateError> {
    let chars: Vec<char> = template.chars().collect();
    let mut out = String::new();
    let mut at = 0;
    for p in find_placeholders(template) {
        out.extend(&chars[at..p.start]);
        match resolve(&p, vals)? {
            Some(v) => out.push_str(&v),
            None => out.extend(&chars[p.start..p.end]),
        }
        at = p.end;
    }
    out.extend(&chars[at..]);
    Ok(out.replace("\\{{", "{{"))
}

/// Instantiate a template **document**: placeholders in the text are replaced (marks
/// stay with their text), variable point marks are filled and become named ranges over
/// the inserted content. The result has fresh ids.
pub fn instantiate(template: &Document, vals: &Values) -> Result<Document, TemplateError> {
    let mut doc = template.clone();
    doc.id = new_id();
    for b in &mut doc.blocks {
        b.id = new_id();
        // 1. variable point marks (descending so earlier offsets stay valid)
        let mut points: Vec<(Uuid, usize, String)> = b
            .marks
            .iter()
            .filter(|m| m.kind == MarkKind::Variable)
            .filter_map(|m| match &m.value {
                MarkValue::Variable { name, .. } => Some((m.id, m.start, name.clone())),
                _ => None,
            })
            .collect();
        points.sort_by_key(|(_, s, _)| std::cmp::Reverse(*s));
        for (id, at, name) in points {
            let Some(v) = vals.vars.get(&name) else {
                if vals.lenient {
                    continue;
                }
                return Err(TemplateError::Missing(name));
            };
            let n = v.chars().count();
            b.splice(at, at, v);
            if let Some(m) = b.marks.get_mut(id) {
                m.start = at;
                m.end = at + n;
            }
        }
        // 2. text placeholders
        let text = b.text.to_string();
        let mut ps = find_placeholders(&text);
        ps.reverse();
        for p in ps {
            if let Some(v) = resolve(&p, vals)? {
                b.splice(p.start, p.end, &v);
            }
        }
        // 3. escaped braces `\{{` -> `{{`
        let chars: Vec<char> = b.text.chars().collect();
        let esc: Vec<usize> = (0..chars.len().saturating_sub(2)).filter(|&i| chars[i] == '\\' && chars[i + 1] == '{' && chars[i + 2] == '{').collect();
        for pos in esc.into_iter().rev() {
            b.splice(pos, pos + 1, "");
        }
        let ms: Vec<Mark> = b.marks.iter().cloned().map(|mut m| (m.id = new_id(), m).1).collect();
        b.marks = MarkSet::from_vec(ms);
    }
    Ok(doc)
}
