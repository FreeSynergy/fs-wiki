//! Container prefixes and tight-list joining.
//!
//! Pattern: Strategy by container kind — `marker` (first line) and `cont_prefix` (following
//! lines) say how each container is written.

use super::options::ExportOptions;
use crate::doc::Container;

/// No blank line between two blocks of the same tight list (or inside one tight item).
pub(super) fn tight_join(a: &[Container], b: &[Container], k: usize) -> bool {
    if k == b.len() {
        return matches!(b.last(), Some(Container::Item { tight: true, .. }));
    }
    let Container::Item {
        tight: true, list, ..
    } = &b[k]
    else {
        return false;
    };
    match a.get(k) {
        Some(Container::Item { list: al, .. }) => al == list,
        Some(_) => false,
        None => k > 0 && matches!(b[k - 1], Container::Item { tight: true, .. }),
    }
}

/// Would two neighbouring list items of different lists read as one list? (Same bullet or same
/// number delimiter.)
pub(super) fn lists_would_merge(a: &Container, b: &Container) -> bool {
    match (a, b) {
        (
            Container::Item {
                list: la,
                number: na,
                marker: ma,
                ..
            },
            Container::Item {
                list: lb,
                number: nb,
                marker: mb,
                ..
            },
        ) => la != lb && na.is_some() == nb.is_some() && ma == mb,
        _ => false,
    }
}

/// Number of shared leading containers (same instance).
pub(super) fn common(a: &[Container], b: &[Container]) -> usize {
    a.iter()
        .zip(b)
        .take_while(|(x, y)| x.inst() == y.inst())
        .count()
}

/// Prefix of the first line of a new container.
pub(super) fn marker(c: &Container, o: ExportOptions) -> String {
    match c {
        Container::Callout { .. } if o.admonitions => "    ".into(),
        Container::Quote { .. } | Container::Callout { .. } => "> ".into(),
        Container::Item {
            number,
            marker,
            task,
            ..
        } => {
            let mut s = match number {
                Some(n) => format!("{n}{marker} "),
                None => format!("{marker} "),
            };
            match task {
                Some(true) => s.push_str("[x] "),
                Some(false) => s.push_str("[ ] "),
                None => {}
            }
            s
        }
        Container::Footnote { label, .. } => format!("[^{label}]: "),
    }
}

/// Prefix of continuation lines of a container.
pub(super) fn cont_prefix(c: &Container, o: ExportOptions) -> String {
    match c {
        Container::Callout { .. } if o.admonitions => "    ".into(),
        Container::Quote { .. } | Container::Callout { .. } => "> ".into(),
        Container::Item { number, .. } => " ".repeat(number.map_or(2, |n| n.to_string().len() + 2)),
        Container::Footnote { .. } => "    ".into(),
    }
}
