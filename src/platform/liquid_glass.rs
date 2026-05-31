//! Liquid Glass utilities for macOS 26+.
//!
//! This module provides full lifecycle management for `NSGlassEffectView`:
//! - **Create**: inserts a new glass view into a GPUI window (no GPUI patches needed)
//! - **Find**: discovers an existing glass view in the native view hierarchy
//! - **Configure**: adjusts corner radius, style, and tint after creation

use gpui::Window;
use objc2::runtime::{AnyClass, NSObjectProtocol};
use objc2_app_kit::NSView;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

const LOG: &str = "liquid_glass";

// NSAutoresizingMaskOptions constants
const NS_VIEW_WIDTH_SIZABLE: u64 = 1 << 1;
const NS_VIEW_HEIGHT_SIZABLE: u64 = 1 << 4;

/// Configuration for `NSGlassEffectView`.
#[derive(Debug, Clone)]
pub struct LiquidGlassConfig {
    /// Corner radius in points.
    pub corner_radius: f64,
    /// Glass style: `0` = default, `1` = clear/thin.
    pub style: i64,
    /// Optional tint color.
    pub tint: Option<LiquidGlassTint>,
    /// Whether to disable the window shadow (recommended for glass windows).
    pub disable_shadow: bool,
}

/// Predefined tint values for `NSGlassEffectView`.
#[derive(Debug, Clone, Copy)]
pub enum LiquidGlassTint {
    /// Neutral grey tint.
    Grey,
    /// System blue tint.
    Blue,
}

impl Default for LiquidGlassConfig {
    fn default() -> Self {
        Self {
            corner_radius: 16.0,
            style: 1, // Clear
            tint: None,
            disable_shadow: true,
        }
    }
}

/// Handle to a discovered or created `NSGlassEffectView`.
pub struct LiquidGlassView {
    view: *mut objc2::runtime::AnyObject,
}

// SAFETY: The pointer comes from the view hierarchy of a live GPUI window.
// We only read from it and send messages; the window owns the view.
unsafe impl Send for LiquidGlassView {}
unsafe impl Sync for LiquidGlassView {}

impl LiquidGlassView {
    /// Set the corner radius.
    pub fn set_corner_radius(&self, radius: f64) {
        // SAFETY: setCornerRadius: is a standard NSGlassEffectView method.
        unsafe {
            let _: () = objc2::msg_send![self.view, setCornerRadius: radius];
        }
    }

    /// Set the glass style (`0` = default, `1` = clear).
    pub fn set_style(&self, style: i64) {
        // SAFETY: setStyle: is a standard NSGlassEffectView method.
        unsafe {
            let _: () = objc2::msg_send![self.view, setStyle: style];
        }
    }

    /// Set the tint color to a predefined tint.
    pub fn set_tint(&self, tint: LiquidGlassTint) {
        // SAFETY: setTintColor: accepts an NSColor or nil.
        unsafe {
            let color: *mut objc2::runtime::AnyObject = match tint {
                LiquidGlassTint::Grey => {
                    let cls =
                        AnyClass::get(c"NSColor").expect("NSColor class should exist");
                    objc2::msg_send![cls, systemGrayColor]
                }
                LiquidGlassTint::Blue => {
                    let cls =
                        AnyClass::get(c"NSColor").expect("NSColor class should exist");
                    objc2::msg_send![cls, systemBlueColor]
                }
            };
            let _: () = objc2::msg_send![self.view, setTintColor: color];
        }
    }

    /// Clear the tint color (pass `nil`).
    pub fn clear_tint(&self) {
        // SAFETY: setTintColor: nil is valid.
        // Use *mut AnyObject (encodes as '@') not *mut u8 (encodes as '*').
        unsafe {
            let _: () = objc2::msg_send![
                self.view,
                setTintColor: std::ptr::null_mut::<objc2::runtime::AnyObject>()
            ];
        }
    }
}

/// Utility for creating, discovering, and configuring `NSGlassEffectView` in a GPUI window.
pub struct LiquidGlass;

impl LiquidGlass {
    /// Create a new `NSGlassEffectView` in the window and apply configuration.
    ///
    /// This is the primary entry point. It:
    /// 1. Extracts the native NSView from the GPUI window
    /// 2. Walks up to the NSWindow
    /// 3. Creates an `NSGlassEffectView` with a layer-backed content view
    /// 4. Inserts it below all other subviews
    /// 5. Optionally disables the window shadow
    ///
    /// Returns `None` if `NSGlassEffectView` is not available (macOS < 26).
    pub fn install(window: &Window, config: &LiquidGlassConfig) -> Option<LiquidGlassView> {
        let glass_cls = AnyClass::get(c"NSGlassEffectView")?;

        let ns_view = Self::get_ns_view(window)?;
        let ns_window = ns_view.window()?;

        // SAFETY: We have a valid NSWindow and NSGlassEffectView class.
        unsafe {
            if config.disable_shadow {
                let _: () = objc2::msg_send![&*ns_window, setHasShadow: false];
            }

            let content_view = ns_window.contentView()?;
            let frame: objc2_foundation::NSRect =
                objc2::msg_send![&*content_view, bounds];

            // Create the glass view.
            let glass: *mut objc2::runtime::AnyObject = objc2::msg_send![glass_cls, alloc];
            let glass: *mut objc2::runtime::AnyObject =
                objc2::msg_send![glass, initWithFrame: frame];
            let _: () = objc2::msg_send![glass, setCornerRadius: config.corner_radius];
            let _: () = objc2::msg_send![glass, setStyle: config.style];

            match &config.tint {
                Some(tint) => {
                    let color: *mut objc2::runtime::AnyObject = match tint {
                        LiquidGlassTint::Grey => {
                            let cls = AnyClass::get(c"NSColor")
                                .expect("NSColor class should exist");
                            objc2::msg_send![cls, systemGrayColor]
                        }
                        LiquidGlassTint::Blue => {
                            let cls = AnyClass::get(c"NSColor")
                                .expect("NSColor class should exist");
                            objc2::msg_send![cls, systemBlueColor]
                        }
                    };
                    let _: () = objc2::msg_send![glass, setTintColor: color];
                }
                None => {
                    let _: () = objc2::msg_send![
                        glass,
                        setTintColor: std::ptr::null_mut::<objc2::runtime::AnyObject>()
                    ];
                }
            }

            // Layer-backed content view — required for the glass effect to render.
            let view_cls = AnyClass::get(c"NSView").expect("NSView class should exist");
            let content: *mut objc2::runtime::AnyObject = objc2::msg_send![view_cls, alloc];
            let content: *mut objc2::runtime::AnyObject =
                objc2::msg_send![content, initWithFrame: frame];
            let _: () = objc2::msg_send![content, setWantsLayer: true];
            let _: () = objc2::msg_send![
                content,
                setAutoresizingMask: NS_VIEW_WIDTH_SIZABLE | NS_VIEW_HEIGHT_SIZABLE
            ];
            let _: () = objc2::msg_send![glass, setContentView: content];

            let _: () = objc2::msg_send![
                glass,
                setAutoresizingMask: NS_VIEW_WIDTH_SIZABLE | NS_VIEW_HEIGHT_SIZABLE
            ];

            // Insert below all other subviews so it acts as a background.
            // NSWindowBelow = -1
            let _: () = objc2::msg_send![
                &*content_view,
                addSubview: glass,
                positioned: -1i64,
                relativeTo: std::ptr::null_mut::<objc2::runtime::AnyObject>()
            ];

            tracing::debug!(target: LOG, "Installed NSGlassEffectView");
            Some(LiquidGlassView { view: glass })
        }
    }

    /// Find an existing `NSGlassEffectView` in the window's native view hierarchy.
    ///
    /// Returns `None` if not found or not available on this macOS version.
    pub fn find(window: &Window) -> Option<LiquidGlassView> {
        let glass_cls = AnyClass::get(c"NSGlassEffectView")?;
        let ns_view = Self::get_ns_view(window)?;

        let result = Self::find_in_subviews(ns_view, glass_cls);
        if result.is_some() {
            tracing::debug!(target: LOG, "Found NSGlassEffectView");
        }
        result
    }

    /// Find an existing glass view and apply a configuration.
    pub fn apply(window: &Window, config: &LiquidGlassConfig) -> bool {
        let Some(glass) = Self::find(window) else {
            return false;
        };
        glass.set_corner_radius(config.corner_radius);
        glass.set_style(config.style);
        match &config.tint {
            Some(tint) => glass.set_tint(*tint),
            None => glass.clear_tint(),
        }
        true
    }

    /// Get the native NSView from a GPUI window.
    fn get_ns_view(window: &Window) -> Option<&NSView> {
        // Use the HasWindowHandle trait method explicitly — Window has an inherent
        // window_handle() that returns AnyWindowHandle, which shadows the trait method.
        let handle = HasWindowHandle::window_handle(window).ok()?;
        let RawWindowHandle::AppKit(appkit) = handle.as_raw() else {
            tracing::debug!(target: LOG, "Not an AppKit window");
            return None;
        };
        // SAFETY: GPUI's AppKit implementation guarantees ns_view is a valid NSView.
        Some(unsafe { appkit.ns_view.cast::<NSView>().as_ref() })
    }

    /// Recursively search the view hierarchy for a view whose class matches.
    fn find_in_subviews(view: &NSView, target: &AnyClass) -> Option<LiquidGlassView> {
        let subviews = view.subviews();
        let count = subviews.count();

        for i in 0..count {
            let subview = subviews.objectAtIndex(i);

            if subview.isKindOfClass(target) {
                return Some(LiquidGlassView {
                    view: subview.as_ref() as *const NSView as *mut objc2::runtime::AnyObject,
                });
            }

            if let Some(found) = Self::find_in_subviews(&subview, target) {
                return Some(found);
            }
        }

        None
    }
}
