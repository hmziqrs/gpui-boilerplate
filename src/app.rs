use es_fluent::EsFluent;
use es_fluent_lang::es_fluent_language;
use gpui::{
    Action, App, AppContext as _, Bounds, Focusable as _, Global, KeyBinding, SharedString,
    WindowBounds, WindowKind, WindowOptions, actions, px, size,
};
use gpui_component::{
    ActiveTheme, Root, TitleBar, WindowExt, scroll::ScrollbarShow, text::markdown,
};
use serde::{Deserialize, Serialize};
use strum::EnumIter;

// ---------------------------------------------------------------------------
// Languages (es-fluent)
// ---------------------------------------------------------------------------

#[es_fluent_language]
#[derive(Clone, Copy, Debug, EnumIter, EsFluent, PartialEq)]
pub enum Languages {}

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

actions!(app, [About, Quit, ToggleSearch]);

#[derive(Action, Clone, PartialEq, Eq, serde::Deserialize)]
#[action(namespace = app, no_json)]
pub struct SelectLocale(pub SharedString);

#[derive(Action, Clone, PartialEq, Eq, serde::Deserialize)]
#[action(namespace = app, no_json)]
pub struct SelectFont(pub usize);

#[derive(Action, Clone, PartialEq, Eq, serde::Deserialize)]
#[action(namespace = app, no_json)]
pub struct SelectRadius(pub usize);

// ---------------------------------------------------------------------------
// Locale state (reactive global for settings page)
// ---------------------------------------------------------------------------

pub const LOCALE_EN: &str = "en";
pub const LOCALE_ZH_CN: &str = "zh-CN";

#[derive(Clone, Debug)]
pub struct LocaleState(pub SharedString);

impl Global for LocaleState {}

pub fn current_locale(cx: &App) -> SharedString {
    cx.global::<LocaleState>().0.clone()
}

pub fn set_locale(locale: &str, cx: &mut App) {
    rust_i18n::set_locale(locale);
    let _ = crate::i18n::i18n().select_language(
        locale
            .parse()
            .unwrap_or_else(|_| es_fluent::unic_langid::langid!("en")),
    );
    cx.set_global::<LocaleState>(LocaleState(SharedString::from(locale.to_string())));
    cx.refresh_windows();
}

pub fn set_theme_mode(mode: gpui_component::ThemeMode, cx: &mut App) {
    gpui_component::Theme::change(mode, None, cx);
    cx.refresh_windows();
}

// ---------------------------------------------------------------------------
// Init
// ---------------------------------------------------------------------------

pub fn init(cx: &mut App) {
    use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt as _};
    let _ = tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::EnvFilter::from_default_env()
                // GPUI logs "window not found" at ERROR when platform display-link
                // callbacks fire after a window is removed — harmless during teardown.
                // GPUI platform callbacks (display link, resize, etc.) fire
                // briefly after a window is removed — "window not found" at
                // ERROR is benign teardown noise, so we silence the module.
                .add_directive("gpui::window=off".parse().unwrap())
                .add_directive(format!("{}=trace", env!("CARGO_PKG_NAME")).parse().unwrap())
                .add_directive("gpui_starter=trace".parse().unwrap())
                .add_directive("user_notify=debug".parse().unwrap())
                .add_directive("notify_rust=debug".parse().unwrap()),
        )
        .try_init();

    // Must be called before using any gpui-component features
    gpui_component::init(cx);

    // Initialize es-fluent i18n for app and form text
    let _ = crate::i18n::init_i18n(
        <_ as Into<es_fluent::unic_langid::LanguageIdentifier>>::into(Languages::default()),
    );

    cx.set_global::<LocaleState>(LocaleState(SharedString::from(
        <_ as Into<es_fluent::unic_langid::LanguageIdentifier>>::into(Languages::default())
            .to_string(),
    )));

    // Restore persisted theme settings
    let persisted =
        std::fs::read_to_string(format!("{}/target/state.json", env!("CARGO_MANIFEST_DIR")))
            .ok()
            .and_then(|json| serde_json::from_str::<PersistedState>(&json).ok());

    // Load extra themes from the themes/ directory (with hot-reload)
    let persisted_for_closure = persisted.clone();
    if let Err(err) = gpui_component::ThemeRegistry::watch_dir(
        std::path::PathBuf::from(format!("{}/themes", env!("CARGO_MANIFEST_DIR"))),
        cx,
        move |cx| {
            if let Some(ref s) = persisted_for_closure
                && let Some(theme) = gpui_component::ThemeRegistry::global(cx)
                    .themes()
                    .get(&s.theme)
                    .cloned()
            {
                gpui_component::Theme::global_mut(cx).apply_config(&theme);
            }
        },
    ) {
        tracing::error!("Failed to watch themes directory: {}", err);
    }

    if let Some(ref s) = persisted
        && let Some(show) = s.scrollbar_show
    {
        gpui_component::Theme::global_mut(cx).scrollbar_show = show;
    }
    cx.refresh_windows();

    // Persist theme on change (only when actually different)
    let last_persisted = persisted.clone();
    cx.observe_global::<gpui_component::Theme>(move |cx| {
        let s = PersistedState {
            theme: cx.theme().theme_name().clone(),
            scrollbar_show: Some(cx.theme().scrollbar_show),
        };
        if Some(&s) != last_persisted.as_ref()
            && let Ok(json) = serde_json::to_string_pretty(&s)
        {
            let _ = std::fs::write(
                format!("{}/target/state.json", env!("CARGO_MANIFEST_DIR")),
                &json,
            );
        }
    })
    .detach();

    // Theme switching actions
    cx.on_action(|switch: &SwitchTheme, cx| {
        if let Some(config) = gpui_component::ThemeRegistry::global(cx)
            .themes()
            .get(&switch.0)
            .cloned()
        {
            gpui_component::Theme::global_mut(cx).apply_config(&config);
        }
        cx.refresh_windows();
    });
    cx.on_action(|switch: &SwitchThemeMode, cx| {
        gpui_component::Theme::change(switch.0, None, cx);
        cx.refresh_windows();
    });
    cx.on_action(|locale: &SelectLocale, cx| {
        set_locale(&locale.0, cx);
    });

    crate::launcher::init(cx);
    cx.set_global(crate::launcher::PendingNavigation(None));
    cx.set_global(crate::launcher::LauncherOpen(false));
    crate::notifications::initialize(cx);

    // Key bindings
    cx.bind_keys([
        KeyBinding::new("cmd-k", ToggleSearch, None),
        KeyBinding::new("/", ToggleSearch, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-q", Quit, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("alt-f4", Quit, None),
    ]);

    cx.on_action(|_: &Quit, cx| {
        cx.quit();
    });

    cx.on_action(|_: &About, cx| {
        if let Some(window) = cx.active_window().and_then(|w| w.downcast::<Root>()) {
            cx.defer(move |cx| {
                window
                    .update(cx, |_, window, cx| {
                        window.defer(cx, |window, cx| {
                            window.open_alert_dialog(cx, |alert, _, _| {
                                alert.title("About").description(markdown(
                                    "GPUI Starter\n\n\
                                    Version 0.1.0\n\n\
                                    A boilerplate for GPUI desktop apps.",
                                ))
                            });
                        });
                    })
                    .unwrap();
            });
        }
    });

    cx.activate(true);
}

// ---------------------------------------------------------------------------
// Window creation
// ---------------------------------------------------------------------------

pub fn create_new_window(title: &str, cx: &mut App) {
    let mut window_size = size(px(1400.0), px(900.0));
    if let Some(display) = cx.primary_display() {
        let display_size = display.bounds().size;
        window_size.width = window_size.width.min(display_size.width * 0.85);
        window_size.height = window_size.height.min(display_size.height * 0.85);
    }
    let window_bounds = Bounds::centered(None, window_size, cx);
    let title = SharedString::from(title.to_string());

    cx.spawn(async move |cx| {
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(window_bounds)),
            titlebar: Some(TitleBar::title_bar_options()),
            window_min_size: Some(gpui::Size {
                width: px(480.),
                height: px(320.),
            }),
            kind: WindowKind::Normal,
            #[cfg(target_os = "linux")]
            window_background: gpui::WindowBackgroundAppearance::Transparent,
            #[cfg(target_os = "linux")]
            window_decorations: Some(gpui::WindowDecorations::Client),
            ..Default::default()
        };

        let window = cx
            .open_window(options, |window, cx| {
                let root_view = cx.new(|cx| crate::root::AppRoot::new(title.clone(), window, cx));

                let focus_handle = root_view.focus_handle(cx);
                window.defer(cx, move |window, cx| {
                    focus_handle.focus(window, cx);
                });

                cx.new(|cx| Root::new(root_view, window, cx))
            })
            .expect("failed to open window");

        window.update(cx, |_, window, _| {
            window.activate_window();
            window.set_window_title(&title);
        })?;

        Ok::<_, anyhow::Error>(())
    })
    .detach();
}

// ---------------------------------------------------------------------------
// Persisted state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct PersistedState {
    theme: SharedString,
    scrollbar_show: Option<ScrollbarShow>,
}

impl Default for PersistedState {
    fn default() -> Self {
        Self {
            theme: "Default Light".into(),
            scrollbar_show: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Re-exported action types used by menus and title_bar
// ---------------------------------------------------------------------------

use gpui_component::ThemeMode;

#[derive(Action, Clone, PartialEq, Eq, serde::Deserialize)]
#[action(namespace = app, no_json)]
pub struct SwitchTheme(pub SharedString);

#[derive(Action, Clone, PartialEq, Eq, serde::Deserialize)]
#[action(namespace = app, no_json)]
pub struct SwitchThemeMode(pub ThemeMode);
