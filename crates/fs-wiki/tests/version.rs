//! The packaged binary says its version without opening a window or a server (the start test of the release).
//!
//! Pattern: Example-based test of the real binary.

use std::process::Command;

#[test]
fn the_version_is_printed_and_nothing_is_started() {
    for flag in ["--version", "-V"] {
        let out = Command::new(env!("CARGO_BIN_EXE_fs-wiki"))
            .arg(flag)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{flag}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let said = String::from_utf8_lossy(&out.stdout);
        assert_eq!(
            said.trim(),
            format!("fs-wiki {}", env!("CARGO_PKG_VERSION"))
        );
    }
}
