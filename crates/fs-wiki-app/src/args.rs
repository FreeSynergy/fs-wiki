//! The command line.
//!
//! Pattern: none — a plain parser, `std::env::args`, no extra crate for the few options.

use std::path::PathBuf;

/// The default folder: the English documentation next to this repo.
pub const DEFAULT_DOCS: &str = "../fs-documentation/docs/en";

/// What was asked for on the command line.
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
