use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;
use tokio::io::Interest;
use tokio::net::{UnixListener, UnixStream};

async fn handle_client(ssh_session: &Session, stream: UnixStream) {
  let mut buffer = [0u8; 1024];
  stream.ready(Interest::READABLE | Interest::WRITABLE).await.unwrap();
  stream.try_read(&mut buffer).unwrap();
  let pane_above = pane_in_direction(ssh_session, Direction::Up).await;
  stream
    .try_write(format!("pane above? {}\n", if pane_above { "yes" } else { "no" }).as_bytes())
    .unwrap();
}

async fn ipc_listener(ssh_session: &Session, socket_path: &Path) {
  std::fs::create_dir_all(socket_path.parent().unwrap()).unwrap();
  let listener = UnixListener::bind(socket_path).unwrap();

  loop {
    let (stream, _addr) = listener.accept().await.unwrap();
    handle_client(ssh_session, stream).await
    // match listener.accept().await {
    //   Ok((stream, _addr)) => {
    //     handle_client(session, stream).await;
    //   }
    //   Err(err) => {
    //     eprintln!("Connection failed: {}", err);
    //   }
    // }
  }
}

use openssh::*;

async fn run_tmux(control_path: &Path) {
  // the openssh library doesn't do pseudo-terminal allocation:
  // https://github.com/openssh-rust/openssh/issues/87
  // So just use ssh command manually instead
  let mut child = tokio::process::Command::new("ssh")
    .arg("-t")
    .arg("-S")
    .arg(control_path)
    // From openssh rust library:
    // ssh does not care about the addr as long as we have passed
    // `-S &*self.ctl`.
    // It is tested on OpenSSH 8.2p1, 8.9p1, 9.0p1
    .arg("none")
    .arg("tmux")
    .stdin(Stdio::inherit())
    .stdout(Stdio::inherit())
    .stderr(Stdio::inherit())
    .spawn()
    .unwrap();
  let _ = child.wait().await.unwrap();
}

#[derive(Copy, Clone, Debug)]
enum Direction {
  Up,
  Down,
  Left,
  Right,
}

async fn pane_in_direction(session: &Session, direction: Direction) -> bool {
  let dir_str = format!("{:?}", direction).to_lowercase();
  let pane_at_top = session
    .command("tmux")
    .arg("display-message")
    .arg("-p")
    .arg(&format!(
      "#{{pane_at_{}}}",
      match direction {
        Direction::Up => "top",
        Direction::Down => "bottom",
        _ => &dir_str,
      }
    ))
    .output()
    .await
    .unwrap()
    .stdout;
  return pane_at_top == b"0\n";
}

#[tokio::main]
async fn main() {
  let args: Vec<_> = std::env::args().collect();
  assert!(args.len() == 2);
  let ssh_path = format!("ssh://{}", args[1]);
  let session = Session::connect_mux(ssh_path, KnownHosts::Strict).await.unwrap();
  let pid = std::process::id();
  // TODO: copy kitty fix
  // TODO: rsync config files
  // TODO: install packages if not installed
  let socket = PathBuf::from_str(&format!("/tmp/ssht/{pid}.sock")).unwrap();

  tokio::select! {
    _ = ipc_listener(&session, &socket) => (),
    _ = run_tmux(session.control_socket()) => ()
  }

  std::fs::remove_file(socket).unwrap();

  session.close().await.unwrap();
}
