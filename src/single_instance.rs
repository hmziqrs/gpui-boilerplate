use std::{
    fs,
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use directories::ProjectDirs;
use gpui::{App, Global};
use interprocess::local_socket::{
    GenericFilePath, GenericNamespaced, ListenerOptions, Stream, prelude::*,
};
use single_instance::SingleInstance;
use std::sync::mpsc;
use std::time::Duration;

use crate::events::{self, AppEventKind};

const INSTANCE_NAME: &str = "com.gpui-starter.app.instance";
const LOG: &str = "gpui_starter::single_instance";
const SCHEME: &str = "gpui-starter://";

pub struct SingleInstanceRuntime {
    _instance: SingleInstance,
    ipc_name: String,
    queue_file: PathBuf,
    ipc_running: Arc<AtomicBool>,
}

impl Global for SingleInstanceRuntime {}

pub struct Preflight {
    pub should_start: bool,
    pub runtime: Option<SingleInstanceRuntime>,
    pub initial_deep_link: Option<String>,
}

pub fn preflight() -> Preflight {
    let args: Vec<String> = std::env::args().collect();
    let deep_link = args.iter().find(|arg| arg.starts_with(SCHEME)).cloned();
    let ipc_name = ipc_name();
    let queue_file = queue_file_path();

    let instance = match SingleInstance::new(INSTANCE_NAME) {
        Ok(instance) => instance,
        Err(err) => {
            eprintln!("single-instance init failed: {err}");
            return Preflight {
                should_start: true,
                runtime: None,
                initial_deep_link: deep_link,
            };
        }
    };

    if instance.is_single() {
        if let Some(parent) = queue_file.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::remove_file(&queue_file);
        Preflight {
            should_start: true,
            runtime: Some(SingleInstanceRuntime {
                _instance: instance,
                ipc_name,
                queue_file,
                ipc_running: Arc::new(AtomicBool::new(true)),
            }),
            initial_deep_link: deep_link,
        }
    } else {
        if let Some(link) = deep_link
            && let Err(err) = send_forwarded_link_via_ipc(&ipc_name, &link)
        {
            tracing::warn!(
                target: LOG,
                error = %err,
                "ipc forward failed; falling back to queue file"
            );
            append_forwarded_link(&queue_file, &link);
        }
        Preflight {
            should_start: false,
            runtime: None,
            initial_deep_link: None,
        }
    }
}

pub fn install(runtime: SingleInstanceRuntime, cx: &mut App) {
    crate::capabilities::set(
        "single_instance",
        crate::capabilities::CapabilityStatus::supported_enabled(),
        cx,
    );
    crate::capabilities::set(
        "second_instance_forwarding",
        crate::capabilities::CapabilityStatus::supported_enabled(),
        cx,
    );
    let queue_file = runtime.queue_file.clone();
    let ipc_name = runtime.ipc_name.clone();
    let ipc_running = runtime.ipc_running.clone();
    cx.set_global(runtime);
    let ipc_ok = start_ipc_forwarder(ipc_name, ipc_running, cx);
    if !ipc_ok {
        crate::capabilities::set(
            "second_instance_forwarding",
            crate::capabilities::CapabilityStatus {
                supported: true,
                enabled: true,
                degraded: true,
                reason: Some("ipc forwarding unavailable; using queue-file fallback".into()),
                last_error: Some("failed to initialize local-socket listener".into()),
            },
            cx,
        );
    }
    start_forwarded_link_poller(queue_file, cx);
}

pub fn shutdown(cx: &mut App) {
    if let Some(runtime) = cx.try_global::<SingleInstanceRuntime>() {
        runtime.ipc_running.store(false, Ordering::SeqCst);
        // Nudge the blocking listener accept loop so it can observe `ipc_running = false`.
        let _ = send_forwarded_link_via_ipc(&runtime.ipc_name, "__shutdown__");
    }
}

fn start_forwarded_link_poller(queue_file: PathBuf, cx: &mut App) {
    tracing::info!(target: LOG, queue = %queue_file.display(), "starting deep-link forwarder poller");
    let bg = cx.background_executor().clone();
    cx.spawn(async move |cx| {
        loop {
            bg.timer(Duration::from_millis(450)).await;
            let links = drain_forwarded_links(&queue_file);
            if links.is_empty() {
                continue;
            }
            cx.update(move |cx| {
                for link in links {
                    tracing::info!(target: LOG, link, "received forwarded deep-link payload");
                    events::emit(AppEventKind::DeepLinkReceived(link), cx);
                }
            });
        }
    })
    .detach();
}

fn start_ipc_forwarder(ipc_name: String, ipc_running: Arc<AtomicBool>, cx: &mut App) -> bool {
    let (tx, rx) = mpsc::channel::<String>();
    let ipc_name_for_thread = ipc_name.clone();
    let thread = std::thread::Builder::new()
        .name("gpui-ipc-forwarder".to_string())
        .spawn(move || {
            let name = match resolve_ipc_name(&ipc_name_for_thread) {
                Ok(name) => name,
                Err(err) => {
                    tracing::error!(
                        target: LOG,
                        error = %err,
                        "failed to resolve ipc listener name"
                    );
                    return;
                }
            };

            let listener = match ListenerOptions::new().name(name).create_sync() {
                Ok(listener) => listener,
                Err(err) => {
                    tracing::error!(target: LOG, error = %err, "failed to create ipc listener");
                    return;
                }
            };

            tracing::info!(target: LOG, ipc = %ipc_name_for_thread, "starting ipc deep-link listener");

            for conn in listener.incoming() {
                if !ipc_running.load(Ordering::SeqCst) {
                    break;
                }
                let Ok(conn) = conn else {
                    continue;
                };
                let mut reader = BufReader::new(conn);
                let mut line = String::new();
                if reader.read_line(&mut line).is_ok() {
                    let link = line.trim().to_string();
                    if !link.is_empty() && link.starts_with(SCHEME) {
                        let _ = tx.send(link);
                    }
                }
            }
        });

    if thread.is_err() {
        return false;
    }

    let bg = cx.background_executor().clone();
    cx.spawn(async move |cx| {
        loop {
            bg.timer(Duration::from_millis(180)).await;
            let mut links = Vec::new();
            while let Ok(link) = rx.try_recv() {
                links.push(link);
            }
            if links.is_empty() {
                continue;
            }
            cx.update(move |cx| {
                for link in links {
                    tracing::info!(target: LOG, link, "received forwarded deep-link payload via ipc");
                    events::emit(AppEventKind::DeepLinkReceived(link), cx);
                }
            });
        }
    })
    .detach();

    true
}

fn append_forwarded_link(path: &PathBuf, link: &str) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    match fs::OpenOptions::new().create(true).append(true).open(path) {
        Ok(mut file) => {
            let _ = writeln!(file, "{link}");
        }
        Err(err) => {
            eprintln!("failed forwarding deep-link to primary instance: {err}");
        }
    }
}

fn send_forwarded_link_via_ipc(ipc_name: &str, link: &str) -> Result<(), String> {
    let name = resolve_ipc_name(ipc_name).map_err(|err| err.to_string())?;
    let mut stream = Stream::connect(name).map_err(|err| err.to_string())?;
    writeln!(stream, "{link}").map_err(|err| err.to_string())
}

fn drain_forwarded_links(path: &PathBuf) -> Vec<String> {
    let Ok(content) = fs::read_to_string(path) else {
        return Vec::new();
    };
    if content.trim().is_empty() {
        return Vec::new();
    }
    let _ = fs::write(path, "");
    content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn queue_file_path() -> PathBuf {
    if let Some(project_dirs) = ProjectDirs::from("com", "gpui-starter", "GPUI Starter") {
        let dir = project_dirs.cache_dir().join("runtime");
        return dir.join("forwarded-deep-links.queue");
    }
    std::env::temp_dir().join("gpui-starter-forwarded-deep-links.queue")
}

fn resolve_ipc_name<'a>(
    ipc_name: &'a str,
) -> std::io::Result<interprocess::local_socket::Name<'a>> {
    if GenericNamespaced::is_supported() {
        ipc_name.to_ns_name::<GenericNamespaced>()
    } else {
        ipc_name.to_fs_name::<GenericFilePath>()
    }
}

fn ipc_name() -> String {
    if GenericNamespaced::is_supported() {
        "com.gpui-starter.app.forwarder".to_string()
    } else {
        queue_file_path()
            .with_extension("sock")
            .display()
            .to_string()
    }
}

#[cfg(test)]
mod tests {
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
}
