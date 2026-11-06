//! The command line.
//!
//! Pattern: Builder-like parser — plain `std::env::args`, no extra crate for six options.

use std::path::PathBuf;

/// The default folder: the English documentation next to this repo.
pub const DEFAULT_DOCS: &str = "../fs-documentation/docs/en";

/// What was asked for on the command line.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Args {
    /// Folder of Markdown files to import.
    pub docs: PathBuf,
    /// Store file; `None` = in memory.
    pub store: Option<PathBuf>,
    /// Draw a picture to this file instead of opening a window.
    pub snapshot: Option<PathBuf>,
    /// Steps for the picture: `open:<path>`, `follow:<href>`, `back`, `search:<term>`.
    pub steps: Vec<String>,
    /// Folder to which the Export action may write (nothing is written without the action).
    pub export: Option<PathBuf>,
    /// Picture size.
    pub size: (u32, u32),
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
            "--store" => out.store = Some(PathBuf::from(value("--store")?)),
            "--snapshot" => out.snapshot = Some(PathBuf::from(value("--snapshot")?)),
            "--export" => out.export = Some(PathBuf::from(value("--export")?)),
            "--step" => out.steps.push(value("--step")?),
            "--size" => out.size = size(&value("--size")?)?,
            "-h" | "--help" => out.help = true,
            other => return Err(format!("unknown option {other}")),
        }
    }
    Ok(out)
}

fn size(text: &str) -> Result<(u32, u32), String> {
    let bad = || format!("--size wants WIDTHxHEIGHT, not {text}");
    let (w, h) = text.split_once('x').ok_or_else(bad)?;
    Ok((w.parse().map_err(|_| bad())?, h.parse().map_err(|_| bad())?))
}

/// The help text.
pub const HELP: &str = "fs-wiki-app: read the documentation in a window

  --docs <folder>     Markdown folder to import (default ../fs-documentation/docs/en)
  --store <file>      keep the wiki in this store file (default: in memory)
  --snapshot <png>    draw a picture instead of opening a window
  --step <step>       with --snapshot, repeatable: open:<path> follow:<href> back search:<term>
  --export <folder>   enable the Export button, writing pages there (e.g. the docs folder)
  --size <WxH>        picture size (default 1200x800)
The folder is only read, never written.";
