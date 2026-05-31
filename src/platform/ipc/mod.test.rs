use super::*;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};

#[test]
fn resolve_socket_path_is_deterministic() {
    let a = IpcEndpoint::new("test-channel");
    let b = IpcEndpoint::new("test-channel");
    assert_eq!(a.socket_path, b.socket_path);
}

#[test]
fn different_names_produce_different_paths() {
    let a = IpcEndpoint::new("alpha");
    let b = IpcEndpoint::new("beta");
    assert_ne!(a.socket_path, b.socket_path);
}

#[tokio::test]
async fn send_and_receive_roundtrip() {
    let unique = format!("ipc-test-{}", uuid::Uuid::new_v4());
    let endpoint = IpcEndpoint::new(&unique);
    let name = endpoint.resolve_name().expect("resolve name");

    let listener = ListenerOptions::new()
        .name(name)
        .create_tokio()
        .expect("create listener");

    let received = Arc::new(std::sync::Mutex::new(String::new()));
    let received_clone = received.clone();

    let server = tokio::spawn(async move {
        let conn = listener.accept().await.expect("accept");
        let mut reader = BufReader::new(&conn);
        let mut line = String::new();
        reader.read_line(&mut line).await.expect("read");
        *received_clone.lock().unwrap() = line.trim().to_string();
    });

    // Give the server a moment to start listening.
    tokio::time::sleep(Duration::from_millis(50)).await;

    endpoint.send("hello ipc").await.expect("send");

    server.await.expect("server join");
    assert_eq!(received.lock().unwrap().as_str(), "hello ipc");
}
