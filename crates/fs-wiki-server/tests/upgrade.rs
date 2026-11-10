//! The real program started on a database written by 0.2.0: it completes the index at start, and
//! every text is listed, page by page.

use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

use fs_interfaces::rpc::{connect, stop_server};
use fs_interfaces::texts_capnp::texts;

/// Kills the program when the test ends, also when it fails.
struct Program(std::process::Child);

impl Drop for Program {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[tokio::test(flavor = "current_thread")]
async fn the_program_repairs_an_old_database_at_start_and_lists_every_text() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let dir = tempfile::tempdir().unwrap();
            let (socket, data) = (dir.path().join("wiki.sock"), dir.path().join("data"));
            std::fs::create_dir_all(&data).unwrap();
            #[cfg(unix)]
            std::fs::set_permissions(&data, std::os::unix::fs::PermissionsExt::from_mode(0o700))
                .unwrap();
            let fixtures =
                Path::new(env!("CARGO_MANIFEST_DIR")).join("../fs-wiki-core/tests/fixtures");
            for (from, to) in [("v020.db", "objects.db"), ("v020.db-wal", "objects.db-wal")] {
                std::fs::copy(fixtures.join(from), data.join(to)).unwrap();
            }
            let child = Command::new(env!("CARGO_BIN_EXE_fs-wiki-server"))
                .arg("--socket")
                .arg(&socket)
                .arg("--data")
                .arg(&data)
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            let mut child = Program(child);
            for _ in 0..400 {
                if connect::<texts::Client>(&socket).await.is_ok() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
            let connection = connect::<texts::Client>(&socket).await.unwrap();
            let mut paths = vec![];
            let mut cursor: Vec<u8> = vec![];
            loop {
                let mut req = connection.client.list_request();
                req.get().init_cursor().set_opaque(&cursor);
                let answer = req
                    .send()
                    .promise
                    .await
                    .expect("the first page of old texts");
                let page = answer.get().unwrap().get_page().unwrap();
                paths.extend(
                    page.get_items()
                        .unwrap()
                        .iter()
                        .map(|t| t.get_path().unwrap().to_string().unwrap()),
                );
                cursor = page.get_next().unwrap().get_opaque().unwrap().to_vec();
                if cursor.is_empty() {
                    break;
                }
            }
            assert_eq!(paths.len(), 5, "{paths:?}");
            drop(connection);
            stop_server(&socket).await.unwrap();
            let mut said = String::new();
            child.0.wait().unwrap();
            std::io::Read::read_to_string(&mut child.0.stderr.take().unwrap(), &mut said).unwrap();
            assert!(said.contains("index completed for 5 texts"), "{said}");
        })
        .await;
}
