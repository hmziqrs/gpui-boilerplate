use std::{io::BufRead, sync::mpsc, time::Duration};

use tempfile::tempdir;

use super::{
    SCHEME, append_forwarded_link, drain_forwarded_links, resolve_ipc_name,
    send_forwarded_link_via_ipc,
};
use interprocess::local_socket::{GenericNamespaced, ListenerOptions, prelude::*};

#[test]
fn forwarded_links_roundtrip_in_order() {
    let dir = tempdir().expect("tempdir");
    let queue = dir.path().join("forward.queue");

    append_forwarded_link(&queue, "gpui-starter://settings");
    append_forwarded_link(&queue, "gpui-starter://notifications");

    let links = drain_forwarded_links(&queue);
    assert_eq!(
        links,
        vec![
            "gpui-starter://settings".to_string(),
            "gpui-starter://notifications".to_string()
        ]
    );
    assert!(drain_forwarded_links(&queue).is_empty());
}

#[test]
fn forwards_link_over_ipc() {
    if !GenericNamespaced::is_supported() {
        return;
    }
    let unique_name = format!("com.gpui-starter.tests.{}", uuid::Uuid::new_v4());
    let name = resolve_ipc_name(&unique_name).expect("resolve name");
    let listener = ListenerOptions::new()
        .name(name)
        .create_sync()
        .expect("create listener");
    let (tx, rx) = mpsc::channel::<String>();
    std::thread::spawn(move || {
        if let Some(Ok(conn)) = listener.incoming().next() {
            let mut reader = std::io::BufReader::new(conn);
            let mut line = String::new();
            let _ = reader.read_line(&mut line);
            let _ = tx.send(line.trim().to_string());
        }
    });

    let sent = format!("{SCHEME}settings");
    send_forwarded_link_via_ipc(&unique_name, &sent).expect("send via ipc");
    let received = rx
        .recv_timeout(Duration::from_secs(3))
        .expect("receive forwarded link");
    assert_eq!(received, sent);
}
