//! What the tests of the real binary share: the temporary HOME with its guard that ends every server a start made.
//!
//! Pattern: Guard (RAII) — `Home` ends the servers of its folder on drop, also after a failed assertion.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

/// The temporary folder of one test. Dropping it ends every server that was started on a socket inside it (also after a
/// failed assertion), waits until they are gone and removes the folder.
pub struct Home(PathBuf);

impl Home {
    pub fn new(name: &str) -> Home {
        let dir = std::env::temp_dir().join(format!("fs-wiki-start-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Home(dir)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Home {
    fn drop(&mut self) {
        for pid in servers_in(&self.0) {
            let _ = Command::new("kill").arg(pid.to_string()).status();
        }
        wait_until_gone(&self.0, Duration::from_secs(10));
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// The processes `fs-wiki serve ...` that have a word of their command line inside `dir` (Linux: `/proc`).
pub fn servers_in(dir: &Path) -> Vec<u32> {
    let Ok(procs) = std::fs::read_dir("/proc") else {
        return Vec::new();
    };
    let dir = dir.to_string_lossy().into_owned();
    procs
        .filter_map(|p| {
            let p = p.ok()?;
            let pid: u32 = p.file_name().to_str()?.parse().ok()?;
            let line = std::fs::read(p.path().join("cmdline")).ok()?;
            let words: Vec<String> = line
                .split(|b| *b == 0)
                .map(|w| String::from_utf8_lossy(w).into_owned())
                .collect();
            let serves = words.iter().any(|w| w == "serve");
            (serves && words.iter().any(|w| w.starts_with(&dir))).then_some(pid)
        })
        .collect()
}

pub fn wait_until_gone(dir: &Path, within: Duration) {
    let began = Instant::now();
    while !servers_in(dir).is_empty() && began.elapsed() < within {
        std::thread::sleep(Duration::from_millis(50));
    }
}
