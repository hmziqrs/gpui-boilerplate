use std::{fs, io::Write, path::PathBuf};

use directories::ProjectDirs;
use gpui::{App, Global};
use single_instance::SingleInstance;
use tokio::time::{Duration, sleep};

use crate::events::{self, AppEventKind};

const INSTANCE_NAME: &str = "com.gpui-starter.app.instance";
const LOG: &str = "gpui_starter::single_instance";
const SCHEME: &str = "gpui-starter://";

pub struct SingleInstanceRuntime {
    _instance: SingleInstance,
    queue_file: PathBuf,
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
                queue_file,
            }),
            initial_deep_link: deep_link,
        }
    } else {
        if let Some(link) = deep_link {
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
    cx.set_global(runtime);
    start_forwarded_link_poller(cx);
}

fn start_forwarded_link_poller(cx: &mut App) {
    let Some(queue_file) = cx
        .try_global::<SingleInstanceRuntime>()
        .map(|runtime| runtime.queue_file.clone())
    else {
        return;
    };

    tracing::info!(target: LOG, queue = %queue_file.display(), "starting deep-link forwarder poller");
    cx.spawn(async move |cx| {
        loop {
            sleep(Duration::from_millis(450)).await;
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
