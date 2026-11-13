//! A server that a window started lets go of everything it inherited from it.
//!
//! Pattern: none (one function) — `Command` closes only what was opened close-on-exec; a descriptor the starting
//! program passed on by hand (a lock of a build tool, a pipe of a test harness) would stay open for as long as the
//! detached server lives, which can be much longer than the window. The server needs only stdio (the starter made
//! it null) and what it opens itself later.

/// Close every descriptor above stderr that this process was started with. Call it first, before anything is opened.
#[cfg(target_os = "linux")]
pub fn close_inherited() {
    use std::os::fd::{FromRawFd, OwnedFd};
    let Ok(dir) = std::fs::read_dir("/proc/self/fd") else {
        return;
    };
    let open: Vec<i32> = dir
        .filter_map(|e| e.ok()?.file_name().to_str()?.parse().ok())
        .collect();
    for fd in open.into_iter().filter(|fd| *fd > 2) {
        // The descriptor of the listing itself is closed by now: only a link that still resolves is an open one.
        if std::fs::read_link(format!("/proc/self/fd/{fd}")).is_ok() {
            // SAFETY: called before this process opens anything of its own and before it has a second thread, so
            // every descriptor above 2 was inherited and nothing else owns it.
            drop(unsafe { OwnedFd::from_raw_fd(fd) });
        }
    }
}

/// Other systems: nothing to do (Windows does not pass handles on by default).
#[cfg(not(target_os = "linux"))]
pub fn close_inherited() {}
