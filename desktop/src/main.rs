mod assets;
mod board;
mod play;
mod puzzles;
mod settings;
mod shell;

use gpui_kit::*;

use crate::{assets::AppAssets, shell::AppShell};

fn main() {
    application().with_assets(AppAssets).run(|cx| {
        init(cx);
        play::bind_keys(cx);

        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(1200.), px(800.)),
                cx,
            ))),
            titlebar: Some(TitlebarOptions {
                title: Some("Chess".into()),
                ..Default::default()
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
