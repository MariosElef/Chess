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

use crate::{assets::AppAssets, components::shell::WINDOW_BUTTONS_AT, views::shell::AppShell};

fn main() {
    application().with_assets(AppAssets).run(|cx| {
        init(cx);
        views::history::bind_keys(cx);
        views::sound::install_speaker(cx);

        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(1200.), px(800.)),
                cx,
            ))),
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
