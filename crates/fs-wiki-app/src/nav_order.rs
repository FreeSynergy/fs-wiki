//! The default order of the entries of a folder (`nav.rs` arranges it by `.pages` and `mkdocs.yml` on top).
//!
//! Pattern: Strategy (a sort key) — the index first, the numbered ones by number, the rest by natural name.

/// The order within a folder: the index first, the numbered ones by number, the rest by natural name.
pub(crate) fn sort_key(i: &crate::nav::Item) -> (u8, u32, Vec<Part>) {
    let index = !i.folder && (i.name == "index.md" || i.name == "README.md");
    (
        u8::from(!index),
        i.number.unwrap_or(u32::MAX),
        natural(&i.name.to_lowercase()),
    )
}

/// A piece of a name for the natural order: numbers compare as numbers.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Part {
    Number(u64),
    Text(String),
}

pub(crate) fn natural(name: &str) -> Vec<Part> {
    let mut out = Vec::new();
    let mut chars = name.chars().peekable();
    while let Some(&c) = chars.peek() {
        let digit = c.is_ascii_digit();
        let mut piece = String::new();
        while let Some(&d) = chars.peek() {
            if d.is_ascii_digit() != digit {
                break;
            }
            piece.push(d);
            chars.next();
        }
        out.push(if digit {
            Part::Number(piece.parse().unwrap_or(u64::MAX))
        } else {
            Part::Text(piece)
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nav::Item;

    #[test]
    fn numbers_order_as_numbers_and_the_index_comes_first() {
        let mut v = ["b10.md", "b2.md", "index.md", "a.md"].map(|n| Item {
            name: n.into(),
            folder: false,
            number: None,
        });
        v.sort_by_key(sort_key);
        let names: Vec<_> = v.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(names, ["index.md", "a.md", "b2.md", "b10.md"]);
    }
}
