// A release build on Windows opens as a window app, without a console window behind it.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod assets;
mod clients;
mod components;
mod enums;
mod models;
mod services;
#[cfg(test)]
mod testing;
mod views;

use gpui_kit::*;

use crate::{
    assets::AppAssets,
    clients::dock_icon::set_dock_icon,
    components::shell::{APP_ICON_PNG, WINDOW_BUTTONS_AT},
    views::shell::AppShell,
};

/// The smallest the window goes: a board of a fair size beside a narrowed panel.
const MIN_WINDOW: Size<Pixels> = size(px(900.), px(620.));

fn main() {
    application().with_assets(AppAssets).run(|cx| {
        init(cx);
        set_dock_icon(APP_ICON_PNG);
        views::history::bind_keys(cx);
        views::sound::install_speaker(cx);

        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(1200.), px(800.)),
                cx,
            ))),
            // Narrower or shorter, the board and its panel no longer fit side by side.
            window_min_size: Some(MIN_WINDOW),
            // The app's own header is the title bar, with the Mac's buttons in it.
            titlebar: Some(TitlebarOptions {
                title: Some("Chess".into()),
                appears_transparent: true,
                traffic_light_position: Some(WINDOW_BUTTONS_AT),
            }),
            ..Default::default()
        };
        open_window(options, cx, |window, cx| {
            cx.new(|cx| AppShell::new(window, cx))
        })
        .expect("the main window opens");
        cx.activate(true);
    });
}
