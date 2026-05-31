use std::time::Duration;

use global_hotkey::GlobalHotKeyEvent;
use gpui::App;
use tray_icon::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

const LOG: &str = "gpui_starter::tray";

// ---------------------------------------------------------------------------
// Tray icon pixel data — 36×36 RGBA magnifying-glass template image
// ---------------------------------------------------------------------------

fn build_icon() -> tray_icon::Icon {
    const SIZE: usize = 36;
    let mut px = vec![0u8; SIZE * SIZE * 4];

    let cx = SIZE as f32 * 0.42;
    let cy = SIZE as f32 * 0.42;
    let r_outer = SIZE as f32 * 0.30;
    let r_inner = r_outer - 3.2;

    let hx0 = cx + r_outer * 0.65;
    let hy0 = cy + r_outer * 0.65;
    let hx1 = SIZE as f32 * 0.86;
    let hy1 = SIZE as f32 * 0.86;

    for y in 0..SIZE {
        for x in 0..SIZE {
            let fx = x as f32 + 0.5;
            let fy = y as f32 + 0.5;

            // Lens ring
            let dx = fx - cx;
            let dy = fy - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            let lens_a: f32 = if dist >= r_inner && dist <= r_outer {
                1.0
            } else if dist < r_inner {
                ((dist - (r_inner - 1.0)) / 1.0).clamp(0.0, 1.0)
            } else {
                (1.0 - (dist - r_outer) / 1.0).clamp(0.0, 1.0)
            };

            // Handle segment
            let ex = hx1 - hx0;
            let ey = hy1 - hy0;
            let len2 = ex * ex + ey * ey;
            let t = ((fx - hx0) * ex + (fy - hy0) * ey) / len2;
            let t = t.clamp(0.0, 1.0);
            let px2 = hx0 + t * ex;
            let py2 = hy0 + t * ey;
            let d_h = ((fx - px2) * (fx - px2) + (fy - py2) * (fy - py2)).sqrt();
            let handle_a: f32 = if d_h <= 1.8 {
                1.0
            } else {
                (1.0 - (d_h - 1.8) / 1.0).clamp(0.0, 1.0)
            };

            let a = (lens_a.max(handle_a) * 255.0) as u8;
            let i = (y * SIZE + x) * 4;
            px[i] = 0;
            px[i + 1] = 0;
            px[i + 2] = 0;
            px[i + 3] = a;
        }
    }

    // SAFETY: Pixel data is generated from a compile-time constant RGBA buffer.
    // The dimensions match the buffer length by construction.
    tray_icon::Icon::from_rgba(px, SIZE as u32, SIZE as u32)
        .expect("tray icon pixel data is valid: compile-time constant")
}

// ---------------------------------------------------------------------------
// Public entry point — call once from main() on macOS
// ---------------------------------------------------------------------------

pub fn setup(cx: &mut App) {
    tracing::info!(target: LOG, "Setting up tray icon");

    let icon = build_icon();
    let Ok(tray) = TrayIconBuilder::new()
        .with_icon(icon)
        .with_icon_as_template(true)
        .with_tooltip("Open Launcher  (⌥Space)")
        .with_menu_on_left_click(false)
        .build()
    else {
        tracing::error!("failed to create system tray icon");
        return;
    };
    Box::leak(Box::new(tray));
    tracing::debug!(target: LOG, "Tray icon created");

    cx.spawn(async move |cx| {
        let bg = cx.background_executor();
        loop {
            while let Ok(ev) = TrayIconEvent::receiver().try_recv() {
                if let TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } = ev
                {
                    tracing::info!(target: LOG, source = "tray_click", "Launcher trigger");
                    cx.update(crate::launcher::open_launcher);
                }
            }

            while GlobalHotKeyEvent::receiver().try_recv().is_ok() {
                tracing::info!(target: LOG, source = "hotkey_alt_space", "Launcher trigger");
                cx.update(crate::launcher::open_launcher);
            }

            bg.timer(Duration::from_millis(50)).await;
        }
    })
    .detach();
}
