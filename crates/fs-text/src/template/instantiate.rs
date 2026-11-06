//! Filling templates.
//!
//! Pattern: Template method — resolve each placeholder, splice it keeping marks, then
//! un-escape `\{{`.

use super::error::TemplateError;
use super::placeholder::{find_placeholders, Placeholder};
use super::values::Values;
use crate::doc::Document;
use crate::id::{new_id, Uuid};
use crate::mark::{Mark, MarkKind, MarkSet, MarkValue};

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
/// # Errors
/// [`TemplateError`] for a missing variable (unless lenient) or a bad date format.
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
/// # Errors
/// [`TemplateError`] for a missing variable (unless lenient) or a bad date format.
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
        let esc: Vec<usize> = (0..chars.len().saturating_sub(2))
            .filter(|&i| chars[i] == '\\' && chars[i + 1] == '{' && chars[i + 2] == '{')
            .collect();
        for pos in esc.into_iter().rev() {
            b.splice(pos, pos + 1, "");
        }
        let ms: Vec<Mark> = b
            .marks
            .iter()
            .cloned()
            .map(|mut m| (m.id = new_id(), m).1)
            .collect();
        b.marks = MarkSet::from_vec(ms);
    }
    Ok(doc)
}
