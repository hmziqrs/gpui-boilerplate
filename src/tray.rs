/// macOS menu bar status icon using tray-icon.
///
/// The tray icon is created on GPUI's main thread, kept alive for the
/// duration of the process.  Click events are polled from a background
/// task and dispatched back to the main thread via AsyncApp::update so
/// they can open the launcher window safely.
use std::time::Duration;

use gpui::App;
use tray_icon::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

// ---------------------------------------------------------------------------
// Icon pixel data – a hand-crafted 36×36 RGBA magnifying-glass template image
// ---------------------------------------------------------------------------

fn build_icon() -> tray_icon::Icon {
    const SIZE: usize = 36;
    let mut px = vec![0u8; SIZE * SIZE * 4];

    let cx = SIZE as f32 / 2.0;
    let cy = SIZE as f32 / 2.0 - 2.0; // lens center, shifted slightly up-left
    let r_outer = 9.5f32; // lens outer radius
    let r_inner = 6.5f32; // lens inner radius (hollow)

    // Handle: a thick diagonal line from bottom-right of the lens
    let handle_x0 = cx + r_outer * 0.6;
    let handle_y0 = cy + r_outer * 0.6;
    let handle_x1 = cx + r_outer * 0.6 + 8.0;
    let handle_y1 = cy + r_outer * 0.6 + 8.0;

    for y in 0..SIZE {
        for x in 0..SIZE {
            let fx = x as f32 + 0.5;
            let fy = y as f32 + 0.5;

            // Distance to lens center
            let dx = fx - cx;
            let dy = fy - cy;
            let dist = (dx * dx + dy * dy).sqrt();

            // Distance to handle segment (for thick line)
            let handle_alpha = {
                let hx = handle_x1 - handle_x0;
                let hy = handle_y1 - handle_y0;
                let len2 = hx * hx + hy * hy;
                let t = ((fx - handle_x0) * hx + (fy - handle_y0) * hy) / len2;
                let t = t.clamp(0.0, 1.0);
                let px2 = handle_x0 + t * hx;
                let py2 = handle_y0 + t * hy;
                let d = ((fx - px2) * (fx - px2) + (fy - py2) * (fy - py2)).sqrt();
                if d < 2.5 { 1.0f32 } else if d < 3.5 { 3.5 - d } else { 0.0 }
            };

            // Lens ring alpha
            let lens_alpha = if dist < r_outer && dist > r_inner {
                1.0f32
            } else if dist <= r_inner && dist >= r_inner - 0.8 {
                // anti-alias inner edge
                (dist - (r_inner - 0.8)) / 0.8
            } else if dist >= r_outer && dist <= r_outer + 0.8 {
                // anti-alias outer edge
                1.0 - (dist - r_outer) / 0.8
            } else {
                0.0
            };

            let a = (lens_alpha.max(handle_alpha) * 255.0) as u8;
            let i = (y * SIZE + x) * 4;
            px[i] = 0;      // R – black for macOS template image
            px[i + 1] = 0;  // G
            px[i + 2] = 0;  // B
            px[i + 3] = a;  // A
        }
    }

    tray_icon::Icon::from_rgba(px, SIZE as u32, SIZE as u32)
        .expect("tray icon pixel data is valid")
}

// ---------------------------------------------------------------------------
// Public entry point – call once from app::init on macOS
// ---------------------------------------------------------------------------

pub fn setup(cx: &mut App) {
    let icon = build_icon();

    let tray: TrayIcon = TrayIconBuilder::new()
        .with_icon(icon)
        .with_icon_as_template(true) // adapts to light/dark menu bar automatically
        .with_tooltip("Open Launcher  (⌘K)")
        .build()
        .expect("failed to create tray icon");

    // Leak the tray icon so it stays alive for the whole process lifetime.
    // This is intentional: the tray icon IS the process's lifetime object.
    Box::leak(Box::new(tray));

    // Poll click events from a background task and dispatch to main thread.
    cx.spawn(async move |cx| {
        let bg = cx.background_executor();
        loop {
            while let Ok(event) = TrayIconEvent::receiver().try_recv() {
                if let TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } = event
                {
                    cx.update(crate::launcher::open_launcher);
                }
            }
            // Poll every 50 ms – low overhead, imperceptible latency
            bg.timer(Duration::from_millis(50)).await;
        }
    })
    .detach();
}
