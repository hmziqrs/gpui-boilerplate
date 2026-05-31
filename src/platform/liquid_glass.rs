//! Liquid Glass utilities for macOS 26+.
//!
//! GPUI creates `NSGlassEffectView` behind the metal layer when
//! `WindowBackgroundAppearance::Blurred` is set, but offers no public API
//! to configure it after creation. This module reaches into the native
//! view hierarchy via `raw-window-handle` to find and reconfigure the
//! glass view directly — no GPUI source patches required.

use gpui::Window;
use objc2::runtime::AnyClass;
use objc2_app_kit::NSView;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

const LOG: &str = "liquid_glass";

/// Configuration for `NSGlassEffectView`.
#[derive(Debug, Clone)]
pub struct LiquidGlassConfig {
    /// Corner radius in points.
    pub corner_radius: f64,
    /// Glass style: `0` = default, `1` = clear/thin.
    pub style: i64,
    /// Optional tint color.
    pub tint: Option<LiquidGlassTint>,
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
        }
    }
}

/// Handle to a discovered `NSGlassEffectView`.
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
        // The view pointer is valid for the lifetime of the window.
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
        // We create a system color via class method calls.
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
        unsafe {
            let _: () =
                objc2::msg_send![self.view, setTintColor: std::ptr::null_mut::<u8>()];
        }
    }
}

/// Utility for discovering and configuring `NSGlassEffectView` in a GPUI window.
pub struct LiquidGlass;

impl LiquidGlass {
    /// Find the `NSGlassEffectView` in a GPUI window's native view hierarchy.
    ///
    /// Returns `None` if:
    /// - The window handle cannot be obtained
    /// - The platform is not macOS
    /// - `NSGlassEffectView` class doesn't exist (macOS < 26)
    /// - The view is not found in the hierarchy
    pub fn find(window: &Window) -> Option<LiquidGlassView> {
        // Check if NSGlassEffectView exists on this macOS version.
        let glass_cls = AnyClass::get(c"NSGlassEffectView")?;

        // Use the HasWindowHandle trait method explicitly — Window has an inherent
        // window_handle() that returns AnyWindowHandle, which shadows the trait method.
        let handle = HasWindowHandle::window_handle(window).ok()?;
        let RawWindowHandle::AppKit(appkit) = handle.as_raw() else {
            tracing::debug!(target: LOG, "Not an AppKit window");
            return None;
        };

        // SAFETY: GPUI's AppKit implementation guarantees ns_view is a valid NSView.
        let ns_view: &NSView = unsafe { appkit.ns_view.cast::<NSView>().as_ref() };

        let result = Self::find_in_subviews(ns_view, glass_cls);
        if result.is_some() {
            tracing::debug!(target: LOG, "Found NSGlassEffectView");
        } else {
            tracing::debug!(target: LOG, "NSGlassEffectView not found in view hierarchy");
        }
        result
    }

    /// Find the `NSGlassEffectView` and apply a configuration.
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

    /// Recursively search the view hierarchy for a view whose class matches.
    fn find_in_subviews(view: &NSView, target: &AnyClass) -> Option<LiquidGlassView> {
        use objc2::runtime::NSObjectProtocol;

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
