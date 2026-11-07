//! The real program: started, used, stopped and started again; the texts are still there.
//!
//! Pattern: Example-based tests of the binary over a real socket and a real store file.

use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use fs_interfaces::rpc::{connect, read_envelope};
use fs_interfaces::texts_capnp::texts;

fn command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_fs-wiki-server"))
}

fn start(socket: &Path, data: &Path) -> Child {
    command()
        .arg("--socket")
        .arg(socket)
        .arg("--data")
        .arg(data)
        .stderr(Stdio::null())
        .spawn()
        .unwrap()
}

async fn wait_for(socket: &Path) {
    for _ in 0..400 {
        if tokio::net::UnixStream::connect(socket).await.is_ok() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("the server did not come up");
}

fn terminate(mut child: Child) {
    let status = Command::new("kill")
        .arg("-TERM")
        .arg(child.id().to_string())
        .status()
        .unwrap();
    assert!(status.success());
    assert!(child.wait().unwrap().success());
}

async fn create(root: &texts::Client, path: &str, markdown: &str) -> String {
    let mut req = root.create_request();
    req.get().set_path(path);
    req.get().set_markdown(markdown);
    let answer = req.send().promise.await.unwrap();
    let env = read_envelope(
        answer
            .get()
            .unwrap()
            .get_text()
            .unwrap()
            .get_envelope()
            .unwrap(),
    );
    env.unwrap().iri.unwrap()
}

async fn stored_paths(socket: &Path) -> Vec<String> {
    let connection = connect::<texts::Client>(socket).await.unwrap();
    let mut req = connection.client.list_request();
    req.get().set_prefix("");
    let answer = req.send().promise.await.unwrap();
    let page = answer.get().unwrap().get_page().unwrap();
    page.get_items()
        .unwrap()
        .iter()
        .map(|t| t.get_path().unwrap().to_string().unwrap())
        .collect()
}

#[tokio::test(flavor = "current_thread")]
async fn texts_survive_a_restart_and_a_second_start_is_refused() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let dir = tempfile::tempdir().unwrap();
            let socket = dir.path().join("wiki.sock");
            let data = dir.path().join("data");

            let first = start(&socket, &data);
            wait_for(&socket).await;
            let mode = std::fs::metadata(&data).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o700, "the data is for this user alone");

            let mut second = start(&socket, &data);
            assert_eq!(
                second.wait().unwrap().code(),
                Some(1),
                "a second server started"
            );
            assert!(socket.exists(), "the second start touched the first");

            let connection = connect::<texts::Client>(&socket).await.unwrap();
            create(&connection.client, "kept/a", "# Kept\n").await;
            drop(connection);

            terminate(first);
            assert!(!socket.exists(), "the socket stayed behind");

            let again = start(&socket, &data);
            wait_for(&socket).await;
            assert_eq!(stored_paths(&socket).await, ["kept/a"]);
            terminate(again);
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn the_instance_names_the_iri_of_a_new_text() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let dir = tempfile::tempdir().unwrap();
            let socket = dir.path().join("wiki.sock");
            let server = command()
                .arg("--socket")
                .arg(&socket)
                .arg("--data")
                .arg(dir.path().join("data"))
                .args(["--instance", "wiki.example.org"])
                .stderr(Stdio::null())
                .spawn()
                .unwrap();
            wait_for(&socket).await;
            let connection = connect::<texts::Client>(&socket).await.unwrap();
            let iri = create(&connection.client, "i", "x\n").await;
            drop(connection);
            // Stopped first: a failed assertion must not leave the server running.
            terminate(server);
            assert!(iri.starts_with("https://wiki.example.org/texts/"), "{iri}");
        })
        .await;
}

#[test]
fn it_refuses_bad_arguments_and_a_data_directory_others_can_read() {
    let run = |args: &[&str]| {
        command()
            .args(args)
            .stderr(Stdio::null())
            .stdout(Stdio::null())
            .status()
            .unwrap()
            .code()
    };
    assert_eq!(run(&["--help"]), Some(0));
    assert_eq!(run(&["--nonsense"]), Some(2));
    assert_eq!(run(&["--socket"]), Some(2));
    assert_eq!(
        run(&["--socket", "/s", "--data", "/d", "--actor", ""]),
        Some(2)
    );
    let dir = tempfile::tempdir().unwrap();
    for bad in [
        "a/b",
        "x y",
        "user@example.org",
        "example.org:80",
        "-a.example",
    ] {
        let code = command()
            .arg("--socket")
            .arg(dir.path().join("x.sock"))
            .arg("--data")
            .arg(dir.path().join("data"))
            .args(["--instance", bad])
            .stderr(Stdio::null())
            .status()
            .unwrap()
            .code();
        assert_eq!(code, Some(2), "{bad}");
    }
    assert!(!dir.path().join("data").exists(), "nothing was opened");

    let open = dir.path().join("open");
    std::fs::create_dir(&open).unwrap();
    std::fs::set_permissions(&open, PermissionsExt::from_mode(0o755)).unwrap();
    let socket = dir.path().join("x.sock");
    let (socket, open) = (socket.to_str().unwrap(), open.to_str().unwrap());
    assert_eq!(run(&["--socket", socket, "--data", open]), Some(1));
}
