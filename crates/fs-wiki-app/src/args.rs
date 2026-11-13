//! The command line.
//!
//! Pattern: none — a plain parser, `std::env::args`, no extra crate for the few options.

use std::path::PathBuf;

/// The default folder: the English documentation next to this repo.
pub const DEFAULT_DOCS: &str = "../fs-documentation/docs/en";

/// What was asked for on the command line.
#[allow(clippy::struct_excessive_bools)] // one flag per option of the command line
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Args {
    /// Folder of Markdown files to import.
    pub docs: PathBuf,
    /// The socket of the running `fs-wiki-server`; none = the default place of the server.
    pub socket: Option<PathBuf>,
    /// The file of the records of imported files; none = one file per docs folder in the data
    /// directory (see `store_path`).
    pub origins: Option<PathBuf>,
    /// Draw a picture to this file instead of opening a window.
    pub snapshot: Option<PathBuf>,
    /// Steps for the picture: `open:<path>`, `follow:<href>`, `back`, `search:<term>`.
    pub steps: Vec<String>,
    /// Folder to which the Export action may write (nothing is written without the action).
    pub export: Option<PathBuf>,
    /// Folder for backups of unsaved drafts (default: the app's data directory).
    pub drafts: Option<PathBuf>,
    /// Picture size.
    pub size: (u32, u32),
    /// Open the documentation reader instead of the Admin window.
    pub reader: bool,
    /// Do not open a window: connect, read the descriptions, list the pages and say what was found.
    pub check: bool,
    /// The facet to show (`--facet NAME`, the link `fs-<facet>`, ADR 0063), read by the parser of `fs-interfaces`.
    pub facet: Option<String>,
    /// The reading wiki (`--read`): the sources below, read only.
    pub read: bool,
    /// The folders to import into one wiki, each below a path prefix (`--source PREFIX=DIR`, `--docs-root`).
    pub sources: Vec<(String, PathBuf)>,
    /// Which sources are language variants of one page family (`--variant PREFIX=GROUP/LANG`, set by `--docs-root`).
    pub variants: Vec<crate::variants::VariantSource>,
    /// The origin language of a family (`--origin-language GROUP=LANG`; default `en`).
    pub origin_langs: Vec<(String, String)>,
    /// The language to read in first (`--lang`; default the one of the environment, else `en`).
    pub lang: Option<String>,
    /// The `mkdocs.yml` whose navigation orders the pages (`--nav`, or the one of `--docs-root`).
    pub nav: Option<PathBuf>,
    /// The face of `--read` for this run (`--frontend a|b|classic`): `a` page tree, head and outline; `b` calm sidebar
    /// and blocks. None = the remembered face of the window's menu.
    pub frontend: Option<String>,
    /// `--index FILE` with `--read`: the heads (id, path, title, revision) of all texts are read from this file when it is there, else listed and written to it (listing the whole documentation takes minutes).
    pub index: Option<PathBuf>,
    /// `--edit` with `--read`: the pages can be edited.
    pub edit: bool,
    /// `--help` was given.
    pub help: bool,
}

/// Parse `args` (without the program name).
///
/// # Errors
/// A message for the person: unknown option or a missing value.
pub fn parse(args: &[String]) -> Result<Args, String> {
    let mut out = Args {
        docs: PathBuf::from(DEFAULT_DOCS),
        size: (1200, 800),
        ..Args::default()
    };
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        let mut value = |name: &str| {
            it.next()
                .cloned()
                .ok_or_else(|| format!("{name} needs a value"))
        };
        match arg.as_str() {
            "--docs" => out.docs = PathBuf::from(value("--docs")?),
            "--socket" => out.socket = Some(PathBuf::from(value("--socket")?)),
            "--origins" => out.origins = Some(PathBuf::from(value("--origins")?)),
            "--store" => {
                return Err(
                    "--store belongs to the server now: start fs-wiki-server with \
                            --data DIR, and give this app its --socket"
                        .into(),
                );
            }
            "--snapshot" => out.snapshot = Some(PathBuf::from(value("--snapshot")?)),
            "--drafts" => out.drafts = Some(PathBuf::from(value("--drafts")?)),
            "--export" => out.export = Some(PathBuf::from(value("--export")?)),
            "--step" => out.steps.push(value("--step")?),
            "--size" => out.size = size(&value("--size")?)?,
            "--reader" => out.reader = true,
            "--read" => out.read = true,
            "--source" => out.sources.push(source(&value("--source")?)?),
            "--docs-root" => docs_root(&mut out, &PathBuf::from(value("--docs-root")?)),
            "--variant" => out.variants.push(variant(&value("--variant")?)?),
            "--origin-language" => out
                .origin_langs
                .push(origin_language(&value("--origin-language")?)?),
            "--lang" => out.lang = Some(language(&value("--lang")?)?),
            "--nav" => out.nav = Some(PathBuf::from(value("--nav")?)),
            "--check" => out.check = true,
            "--edit" => out.edit = true,
            "--index" => out.index = Some(PathBuf::from(value("--index")?)),
            "--frontend" => {
                let v = value("--frontend")?;
                if crate::face::Face::parse(&v).is_none() {
                    return Err(format!("--frontend wants a, b or classic, not {v}"));
                }
                out.frontend = Some(v);
            }
            "--facet" => out.facet = Some(fs_interfaces::rpc::parse_facet(&value("--facet")?)?),
            "-h" | "--help" => out.help = true,
            other => return Err(format!("unknown option {other}")),
        }
    }
    Ok(out)
}

/// `PREFIX=DIR`: the prefix is a path of names with `/` between them.
fn source(text: &str) -> Result<(String, PathBuf), String> {
    let bad = || format!("--source wants PREFIX=FOLDER (prefix like en or manual/de), not {text}");
    let (prefix, dir) = text.split_once('=').ok_or_else(bad)?;
    let fine = !prefix.is_empty()
        && prefix.split('/').all(|p| {
            !p.is_empty()
                && p != "."
                && p != ".."
                && !p.contains(['#', '\\'])
                && !p.chars().any(char::is_control)
        });
    if !fine || dir.is_empty() {
        return Err(bad());
    }
    Ok((prefix.to_owned(), PathBuf::from(dir)))
}

/// `PREFIX=GROUP/LANG`: the source at `PREFIX` holds the language `LANG` of the pages of `GROUP`.
fn variant(text: &str) -> Result<crate::variants::VariantSource, String> {
    let bad = || format!("--variant wants PREFIX=GROUP/LANG (en=docs/en), not {text}");
    let (prefix, rest) = text.split_once('=').ok_or_else(bad)?;
    let (group, lang) = rest.rsplit_once('/').ok_or_else(bad)?;
    if prefix.is_empty() || group.is_empty() {
        return Err(bad());
    }
    Ok(crate::variants::VariantSource {
        prefix: prefix.to_owned(),
        group: group.to_owned(),
        lang: language(lang)?,
    })
}

/// `GROUP=LANG`.
fn origin_language(text: &str) -> Result<(String, String), String> {
    let (group, lang) = text
        .split_once('=')
        .filter(|(g, _)| !g.is_empty())
        .ok_or_else(|| format!("--origin-language wants GROUP=LANG (adr=de), not {text}"))?;
    Ok((group.to_owned(), language(lang)?))
}

/// A language code: lower-case letters and `-` (`de`, `en`, `pt-br`).
fn language(text: &str) -> Result<String, String> {
    if !text.is_empty() && text.chars().all(|c| c.is_ascii_lowercase() || c == '-') {
        Ok(text.to_owned())
    } else {
        Err(format!("a language is written like de or en, not {text}"))
    }
}

/// The checkout of the documentation: its four sources and its `mkdocs.yml`.
fn docs_root(out: &mut Args, root: &std::path::Path) {
    // prefix, folder, family, language: English and German of `docs` are one page; the decisions are German only (and
    // their origin), the research English only.
    for (prefix, folder, group, lang) in [
        ("en", "docs/en", "docs", "en"),
        ("de", "docs/de", "docs", "de"),
        ("adr", "conventions/decisions", "adr", "de"),
        ("research", "temp/research", "research", "en"),
    ] {
        let dir = root.join(folder);
        if dir.is_dir() {
            out.sources.push((prefix.to_owned(), dir));
            out.variants.push(crate::variants::VariantSource {
                prefix: prefix.to_owned(),
                group: group.to_owned(),
                lang: lang.to_owned(),
            });
        }
    }
    out.origin_langs.push(("adr".to_owned(), "de".to_owned()));
    let mkdocs = root.join("mkdocs.yml");
    if mkdocs.is_file() && out.nav.is_none() {
        out.nav = Some(mkdocs);
    }
}

fn size(text: &str) -> Result<(u32, u32), String> {
    let bad = || format!("--size wants WIDTHxHEIGHT, not {text}");
    let (w, h) = text.split_once('x').ok_or_else(bad)?;
    Ok((w.parse().map_err(|_| bad())?, h.parse().map_err(|_| bad())?))
}

/// The help text.
pub const HELP: &str = "fs-wiki-app: the wiki in a window (Admin view of pages, folders, history, trash and links)

  --read              the reading wiki: the sources below in one wiki, read only (sidebar, page, search)
  --docs-root <dir>   with --read: a checkout of the documentation (docs/en as en/, docs/de as de/,
                      conventions/decisions as adr/, temp/research as research/, its mkdocs.yml for the order)
  --source <p>=<dir>  with --read, repeatable: import <dir> below the path prefix <p>
  --variant <p>=<g>/<l>  with --read, repeatable: the source at <p> is the language <l> of the pages of family <g>;
                      sources of one family with equal relative paths are ONE page (en/a.md and de/a.md)
  --origin-language <g>=<l>  the language a family's translations are made from (default en)
  --lang <l>          the language to read in first (default: that of the environment, else en)
  --frontend a|b|classic  with --read: the face for this run (not remembered): a = page tree, head and outline;
                      b = calm sidebar and block editor; classic = the plain reader. Without it the window
                      starts with the face remembered from the menu View > Face (first start: a)
  --edit              with --read: the pages can be edited
  --index <file>      with --read: read the heads of all texts from this file (else list them once and write it)
  --nav <mkdocs.yml>  with --read: the navigation that orders the pages (and the .pages files in the folders)
  --reader            open the documentation reader and editor instead (the options below belong to it)
  --facet <name>      the facet to show (the link fs-<facet> sets it); the server it starts gets it too
  --check             no window: connect, read the descriptions and the pages, and say what was found

  --socket <path>     socket of the running fs-wiki-server (default $XDG_RUNTIME_DIR/freesynergy/wiki.sock);
                      start it first: fs-wiki-server [--socket <path>] [--data <dir>]
  --docs <folder>     Markdown folder to import (default ../fs-documentation/docs/en)
  --origins <file>    keep the records of imported files here (default: one file per docs folder in
                      the data directory, ~/.local/share/freesynergy/fs-wiki)
  --snapshot <png>    draw a picture instead of opening a window
  --step <step>       with --snapshot, repeatable: open:<path> follow:<href> back search:<term>
  --drafts <folder>   where unsaved drafts are backed up (default ~/.local/share/fs-wiki/drafts)
  --export <folder>   enable the Export button, writing pages there (e.g. the docs folder)
  --size <WxH>        picture size (default 1200x800)
The folder is only read, never written. The wiki itself (the texts) lives in the server; one server
is one wiki, so use one server (its own --data) per documentation source.";
