//! The order of the pages of the sources, as the documentation's own navigation says it.
//!
//! Pattern: Builder (reads the navigation files, builds one rank per page) — `mkdocs.yml` (`nav:` and the numbered
//! `nav_translations`) and the `.pages` files of the awesome-pages plugin (`title:`, `nav:` with `...`). The reader lists its
//! pages in the order it is given, so the rank decides the order of the sidebar. The folders' titles of `.pages` cannot be shown:
//! the sidebar names a folder by its path part (a limit of the reader of `fs-components`, in `DESIGN.md`); the number in
//! such a title (`3 Components`) still orders the folders. A page or folder no navigation mentions comes after the listed ones,
//! by name (natural order: `2` before `10`), the `index` of a folder first.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use yaml_serde::Value;

/// The rank of every page (its text path, with the prefix of its source).
#[derive(Clone, Debug, Default)]
pub struct Nav {
    rank: HashMap<String, usize>,
}

impl Nav {
    /// The rank of the page at `path`; none = not known (it comes last).
    pub fn rank(&self, path: &str) -> Option<usize> {
        self.rank.get(path).copied()
    }

    /// How many pages are ranked.
    pub fn len(&self) -> usize {
        self.rank.len()
    }

    /// Whether no page is ranked.
    pub fn is_empty(&self) -> bool {
        self.rank.is_empty()
    }

    /// The order of every `.md` file below the `sources` (`(prefix, folder)`); `mkdocs` is the `mkdocs.yml` whose `nav:` and
    /// numbered `nav_translations` count too. What cannot be read is said in the second part, never dropped silently.
    pub fn read(sources: &[(String, PathBuf)], mkdocs: Option<&Path>) -> (Self, Vec<String>) {
        let mut notes = Vec::new();
        let config = mkdocs.and_then(|f| match std::fs::read_to_string(f) {
            Ok(text) => match yaml_serde::from_str::<Value>(&text) {
                Ok(v) => Some(v),
                Err(e) => {
                    notes.push(format!("{}: the navigation is not read: {e}", f.display()));
                    None
                }
            },
            Err(e) => {
                notes.push(format!("{}: the navigation is not read: {e}", f.display()));
                None
            }
        });
        let numbers = config.as_ref().map(numbered_titles).unwrap_or_default();
        let listed = config
            .as_ref()
            .and_then(|c| c.get("nav"))
            .map(listed_files)
            .unwrap_or_default();
        let mut rank = HashMap::new();
        // One count over all sources, in the order they were given: the first source's pages come first.
        let mut next = 0;
        for (prefix, root) in sources {
            let mut order = Vec::new();
            for f in &listed {
                if root.join(f).is_file() && !order.contains(f) {
                    order.push(f.clone());
                }
            }
            let mut walk = Walk {
                root,
                numbers: &numbers,
                notes: &mut notes,
            };
            let rest = walk.dir("");
            let rest: Vec<String> = rest.into_iter().filter(|f| !order.contains(f)).collect();
            order.extend(rest);
            let below = if prefix.is_empty() {
                String::new()
            } else {
                format!("{prefix}/")
            };
            for f in order {
                rank.insert(format!("{below}{f}"), next);
                next += 1;
            }
        }
        (Self { rank }, notes)
    }
}

/// The numbers the configuration gives to names: `nav_translations: {"6 Manifesto": …}` says that `manifesto` is sixth.
fn numbered_titles(config: &Value) -> HashMap<String, u32> {
    let mut out = HashMap::new();
    let mut stack = vec![config];
    while let Some(v) = stack.pop() {
        match v {
            Value::Mapping(m) => {
                for (k, x) in m {
                    if k.as_str() == Some("nav_translations")
                        && let Some(t) = x.as_mapping()
                    {
                        for key in t.keys().filter_map(Value::as_str) {
                            if let Some((n, name)) = split_number(key) {
                                out.entry(name.to_lowercase().replace(' ', "-"))
                                    .or_insert(n);
                            }
                        }
                    }
                    stack.push(x);
                }
            }
            Value::Sequence(s) => stack.extend(s),
            _ => {}
        }
    }
    out
}

/// `"3 Components"` is (3, `Components`).
fn split_number(title: &str) -> Option<(u32, &str)> {
    let (n, rest) = title.split_once(' ')?;
    Some((n.parse().ok()?, rest.trim()))
}

/// The files a `nav:` list names, in its order (titles and nesting do not matter here).
fn listed_files(nav: &Value) -> Vec<String> {
    let mut out = Vec::new();
    match nav {
        Value::Sequence(items) => {
            for item in items {
                out.extend(listed_files(item));
            }
        }
        Value::Mapping(m) => {
            for (_, v) in m {
                out.extend(listed_files(v));
            }
        }
        Value::String(s) if is_md(s) => out.push(s.trim_start_matches("./").to_string()),
        _ => {}
    }
    out
}

/// A Markdown file by its name: written `.md`, lower case, as the import wants it.
fn is_md(name: &str) -> bool {
    Path::new(name).extension().is_some_and(|e| e == "md")
}

struct Walk<'a> {
    root: &'a Path,
    numbers: &'a HashMap<String, u32>,
    notes: &'a mut Vec<String>,
}

/// One entry of a folder, for ordering.
struct Item {
    /// The name in the folder (`start`, `index.md`).
    name: String,
    folder: bool,
    /// The number its title or the configuration gives it.
    number: Option<u32>,
}

impl Walk<'_> {
    /// The `.md` files below the folder `rel` (relative to the source, `""` = the root), in order.
    fn dir(&mut self, rel: &str) -> Vec<String> {
        let here = if rel.is_empty() {
            self.root.to_path_buf()
        } else {
            self.root.join(rel)
        };
        let join = |name: &str| {
            if rel.is_empty() {
                name.to_string()
            } else {
                format!("{rel}/{name}")
            }
        };
        let Ok(read) = std::fs::read_dir(&here) else {
            return Vec::new();
        };
        let pages = self.pages_file(&here);
        let mut items: Vec<Item> = read
            .flatten()
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                let kind = e.file_type().ok()?;
                if kind.is_symlink() || name.starts_with('.') {
                    return None;
                }
                if kind.is_dir() {
                    let title = self.pages_file(&e.path()).title;
                    let number = title.as_deref().and_then(split_number).map(|(n, _)| n);
                    Some(Item {
                        name,
                        folder: true,
                        number,
                    })
                } else if kind.is_file() && is_md(&name) {
                    let number = self.numbers.get(name.trim_end_matches(".md")).copied();
                    Some(Item {
                        name,
                        folder: false,
                        number,
                    })
                } else {
                    None
                }
            })
            .collect();
        items.sort_by_key(sort_key);
        let names: Vec<String> = items.iter().map(|i| i.name.clone()).collect();
        let order = arrange(&names, &pages.nav);
        let mut out = Vec::new();
        for name in order {
            let Some(item) = items.iter().find(|i| i.name == name) else {
                continue;
            };
            if item.folder {
                out.extend(self.dir(&join(&name)));
            } else {
                out.push(join(&name));
            }
        }
        out
    }

    /// The `.pages` of a folder (empty when there is none or it does not read; the latter is noted).
    fn pages_file(&mut self, dir: &Path) -> Pages {
        let file = dir.join(".pages");
        let Ok(text) = std::fs::read_to_string(&file) else {
            return Pages::default();
        };
        match yaml_serde::from_str::<Value>(&text) {
            Ok(v) => Pages {
                title: v.get("title").and_then(Value::as_str).map(str::to_owned),
                nav: v.get("nav").map(nav_names).unwrap_or_default(),
            },
            Err(e) => {
                self.notes.push(format!(
                    "{}: the navigation is not read: {e}",
                    file.display()
                ));
                Pages::default()
            }
        }
    }
}

#[derive(Default)]
struct Pages {
    title: Option<String>,
    /// The names of `nav:` (`...` is the rest), without titles.
    nav: Vec<String>,
}

/// The names of an awesome-pages `nav:` (`file.md`, `folder`, `Title: file.md`, `...`).
fn nav_names(nav: &Value) -> Vec<String> {
    let Value::Sequence(items) = nav else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|i| match i {
            Value::String(s) => Some(s.clone()),
            Value::Mapping(m) => m.values().find_map(Value::as_str).map(str::to_owned),
            _ => None,
        })
        .collect()
}

/// The order within a folder: the index first, the numbered ones by number, the rest by natural name.
fn sort_key(i: &Item) -> (u8, u32, Vec<Part>) {
    let index = !i.folder && (i.name == "index.md" || i.name == "README.md");
    (
        u8::from(!index),
        i.number.unwrap_or(u32::MAX),
        natural(&i.name.to_lowercase()),
    )
}

/// A piece of a name for the natural order: numbers compare as numbers.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum Part {
    Number(u64),
    Text(String),
}

fn natural(name: &str) -> Vec<Part> {
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

/// `names` (already in default order) arranged by a `.pages` `nav:`: the listed ones in its order, `...` stands for the
/// rest, and without it the rest follows the listed ones (nothing is hidden). A listed name may omit `.md`.
fn arrange(names: &[String], nav: &[String]) -> Vec<String> {
    if nav.is_empty() {
        return names.to_vec();
    }
    let find = |n: &str| {
        names
            .iter()
            .find(|x| x.as_str() == n || x.strip_suffix(".md") == Some(n))
            .cloned()
    };
    let mut out: Vec<String> = Vec::new();
    let mut rest_at = None;
    for n in nav {
        if n == "..." {
            rest_at = Some(out.len());
        } else if let Some(x) = find(n)
            && !out.contains(&x)
        {
            out.push(x);
        }
    }
    let rest: Vec<String> = names.iter().filter(|n| !out.contains(n)).cloned().collect();
    let at = rest_at.unwrap_or(out.len());
    out.splice(at..at, rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn a_pages_nav_lists_first_and_dots_stand_for_the_rest() {
        let names: Vec<String> = ["a.md", "b.md", "c.md", "d.md"].map(String::from).to_vec();
        let nav = ["c", "...", "a.md"].map(String::from).to_vec();
        assert_eq!(arrange(&names, &nav), ["c.md", "b.md", "d.md", "a.md"]);
        let nav = ["d.md"].map(String::from).to_vec();
        assert_eq!(arrange(&names, &nav), ["d.md", "a.md", "b.md", "c.md"]);
    }
}
