use gpui::{App, Entity, Menu, MenuItem, SharedString};
use gpui_component::{
    ActiveTheme as _, GlobalState, Theme, ThemeMode, ThemeRegistry, menu::AppMenuBar,
};

use crate::{
    app::{About, ExecuteCommand, Quit, SelectLocale, SwitchTheme, SwitchThemeMode},
    commands::{self, CommandId},
};

pub fn init(title: impl Into<SharedString>, cx: &mut App) -> Entity<AppMenuBar> {
    let app_menu_bar = AppMenuBar::new(cx);
    let title: SharedString = title.into();
    update_app_menu(title.clone(), app_menu_bar.clone(), cx);

    cx.on_action({
        let title = title.clone();
        let app_menu_bar = app_menu_bar.clone();
        move |_: &SelectLocale, cx| {
            update_app_menu(title.clone(), app_menu_bar.clone(), cx);
        }
    });

    cx.observe_global::<Theme>({
        let title = title.clone();
        let app_menu_bar = app_menu_bar.clone();
        move |cx| {
            update_app_menu(title.clone(), app_menu_bar.clone(), cx);
        }
    })
    .detach();
    cx.observe_global::<crate::undo_stack::UndoState>({
        let title = title.clone();
        let app_menu_bar = app_menu_bar.clone();
        move |cx| {
            update_app_menu(title.clone(), app_menu_bar.clone(), cx);
        }
    })
    .detach();

    app_menu_bar
}

fn update_app_menu(title: impl Into<SharedString>, app_menu_bar: Entity<AppMenuBar>, cx: &mut App) {
    let title: SharedString = title.into();

    cx.set_menus(build_menus(title.clone(), cx));
    let menus = build_menus(title, cx)
        .into_iter()
        .map(|menu| menu.owned())
        .collect();
    GlobalState::global_mut(cx).set_app_menus(menus);

    app_menu_bar.update(cx, |menu_bar, cx| {
        menu_bar.reload(cx);
    });
}

fn build_menus(title: impl Into<SharedString>, cx: &App) -> Vec<Menu> {
    vec![
        Menu {
            name: title.into(),
            items: vec![
                MenuItem::action("About", About),
                MenuItem::Separator,
                MenuItem::Submenu(Menu {
                    name: "Appearance".into(),
                    items: vec![
                        MenuItem::action("Light", SwitchThemeMode(ThemeMode::Light))
                            .checked(!cx.theme().mode.is_dark()),
                        MenuItem::action("Dark", SwitchThemeMode(ThemeMode::Dark))
                            .checked(cx.theme().mode.is_dark()),
                    ],
                    disabled: false,
                }),
                theme_menu(cx),
                language_menu(cx),
                MenuItem::Separator,
                MenuItem::action("Quit", Quit),
            ],
            disabled: false,
        },
        Menu {
            name: "Edit".into(),
            items: vec![
                MenuItem::action("Undo", ExecuteCommand(CommandId::Undo))
                    .disabled(!commands::availability(CommandId::Undo, cx).enabled),
                MenuItem::action("Redo", ExecuteCommand(CommandId::Redo))
                    .disabled(!commands::availability(CommandId::Redo, cx).enabled),
                MenuItem::separator(),
                MenuItem::action("Cut", gpui_component::input::Cut),
                MenuItem::action("Copy", gpui_component::input::Copy),
                MenuItem::action("Paste", gpui_component::input::Paste),
                MenuItem::separator(),
                MenuItem::action("Select All", gpui_component::input::SelectAll),
            ],
            disabled: false,
        },
        Menu {
            name: "Window".into(),
            items: vec![
                MenuItem::action("Toggle Search", crate::app::ToggleSearch),
                MenuItem::action("Diagnostics", ExecuteCommand(CommandId::OpenDiagnostics)),
            ],
            disabled: false,
        },
        Menu {
            name: "Go".into(),
            items: command_menu_items(
                &[
                    CommandId::OpenHome,
                    CommandId::OpenForm,
                    CommandId::OpenSettings,
                    CommandId::OpenNotifications,
                    CommandId::OpenDiagnostics,
                    CommandId::OpenAbout,
                    CommandId::CopyDiagnostics,
                    CommandId::OpenLogsFolder,
                    CommandId::OpenConfigFolder,
                ],
                cx,
            ),
            disabled: false,
        },
        Menu {
            name: "Tools".into(),
            items: command_menu_items(
                &[CommandId::StartDemoTask, CommandId::CheckConnectivity],
                cx,
            ),
            disabled: false,
        },
    ]
}

fn language_menu(_: &App) -> MenuItem {
    let locale = rust_i18n::locale().to_string();
    MenuItem::Submenu(Menu {
        name: "Language".into(),
        items: vec![
            MenuItem::action("English", SelectLocale("en".into())).checked(locale == "en"),
            MenuItem::action("简体中文", SelectLocale("zh-CN".into())).checked(locale == "zh-CN"),
        ],
        disabled: false,
    })
}

fn theme_menu(cx: &App) -> MenuItem {
    let themes = ThemeRegistry::global(cx).sorted_themes();
    let current_name = cx.theme().theme_name();
    MenuItem::Submenu(Menu {
        name: "Theme".into(),
        items: themes
            .iter()
            .map(|theme| {
                let checked = current_name == &theme.name;
                MenuItem::action(theme.name.clone(), SwitchTheme(theme.name.clone()))
                    .checked(checked)
            })
            .collect(),
        disabled: false,
    })
}

fn command_menu_items(command_ids: &[CommandId], cx: &App) -> Vec<MenuItem> {
    let specs = commands::registry();
    command_ids
        .iter()
        .filter_map(|id| {
            specs.iter().find(|spec| spec.id == *id).map(|spec| {
                let availability = commands::availability(*id, cx);
                let title = if let Some(reason) = availability.disabled_reason {
                    format!("{} ({})", spec.title, reason)
                } else {
                    spec.title.to_string()
                };
                MenuItem::action(title, ExecuteCommand(*id)).disabled(!availability.enabled)
            })
        })
        .collect()
}
