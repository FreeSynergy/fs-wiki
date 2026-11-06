//! Grapheme-cluster boundaries (UAX #29) over char offsets.
//!
//! Pattern: Strategy-free pure functions + Adapter over `unicode-segmentation` — callers
//! ask "where may a mark or edit start / end?" and never see byte offsets or the crate.
//! A user-perceived character (an emoji with skin tone, a ZWJ family, `e` + combining acute,
//! a flag, `\r\n`) is one unit: nobody can mark or cut half of it.

use unicode_segmentation::UnicodeSegmentation;

/// Which way an offset inside a cluster moves to reach a boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Snap {
    /// To the start of the cluster.
    Down,
    /// To the end of the cluster.
    Up,
    /// To the closer boundary (ties go down).
    Nearest,
}

/// Char offsets of all cluster boundaries of `s`, including `0` and the char length.
#[must_use]
pub fn boundaries(s: &str) -> Vec<usize> {
    let mut out = Vec::with_capacity(s.len() + 1);
    let mut at = 0;
    for g in s.graphemes(true) {
        out.push(at);
        at += g.chars().count();
    }
    out.push(at);
    out
}

/// Is `offset` a cluster boundary of `s`? Offsets past the end are not.
#[must_use]
pub fn is_boundary(s: &str, offset: usize) -> bool {
    boundaries(s).binary_search(&offset).is_ok()
}

/// Move `offset` (clamped to the length) to a cluster boundary of `s`.
#[must_use]
pub fn snap(s: &str, offset: usize, dir: Snap) -> usize {
    snap_in(&boundaries(s), offset, dir)
}

/// Snap a range outward: start down, end up. An empty range (a position) snaps to the
/// nearest boundary instead and stays empty.
#[must_use]
pub fn snap_range(s: &str, start: usize, end: usize) -> (usize, usize) {
    let b = boundaries(s);
    if start >= end {
        let p = snap_in(&b, start.min(end), Snap::Nearest);
        return (p, p);
    }
    (snap_in(&b, start, Snap::Down), snap_in(&b, end, Snap::Up))
}

fn snap_in(b: &[usize], offset: usize, dir: Snap) -> usize {
    let len = b.last().copied().unwrap_or(0);
    let o = offset.min(len);
    let i = b.partition_point(|&x| x < o);
    if b.get(i) == Some(&o) {
        return o;
    }
    let (lo, hi) = (b[i - 1], b[i]);
    match dir {
        Snap::Down => lo,
        Snap::Up => hi,
        Snap::Nearest => {
            if o - lo <= hi - o {
                lo
            } else {
                hi
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const THUMB: &str = "👍🏽"; // 2 chars: emoji + skin tone
    const FAMILY: &str = "👨\u{200d}👩\u{200d}👧"; // 5 chars: ZWJ sequence
    const ACUTE: &str = "e\u{301}"; // 2 chars: e + combining acute
    const FLAG: &str = "🇩🇪"; // 2 chars: regional indicators

    #[test]
    fn clusters_are_single_units() {
        assert_eq!(boundaries(&format!("a{THUMB}b")), [0, 1, 3, 4]);
        assert_eq!(boundaries(&format!("a{FAMILY}b")), [0, 1, 6, 7]);
        assert_eq!(boundaries(&format!("{ACUTE}x")), [0, 2, 3]);
        assert_eq!(boundaries(&format!("{FLAG}{FLAG}")), [0, 2, 4]);
        assert_eq!(boundaries("a\r\nb"), [0, 1, 3, 4]);
        assert_eq!(boundaries(""), [0]);
    }

    #[test]
    fn snapping_directions() {
        let s = format!("a{FAMILY}b");
        assert_eq!(snap(&s, 3, Snap::Down), 1);
        assert_eq!(snap(&s, 3, Snap::Up), 6);
        assert_eq!(snap(&s, 3, Snap::Nearest), 1);
        assert_eq!(snap(&s, 5, Snap::Nearest), 6);
        assert_eq!(snap(&s, 99, Snap::Down), 7);
        assert!(is_boundary(&s, 6) && !is_boundary(&s, 2) && !is_boundary(&s, 99));
    }

    #[test]
    fn ranges_grow_and_positions_stay_empty() {
        let s = format!("a{THUMB}b");
        assert_eq!(snap_range(&s, 2, 3), (1, 3));
        assert_eq!(snap_range(&s, 0, 2), (0, 3));
        assert_eq!(snap_range(&s, 2, 2), (1, 1));
        assert_eq!(snap_range(&s, 5, 1), (1, 1));
    }
}
