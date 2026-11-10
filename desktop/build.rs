// A Windows build carries the app's icon inside Chess.exe, as resource 1: GPUI gives that to its
// windows, so it shows in the title bar and the taskbar, and Explorer and the Start menu show it
// too. A Mac app's icon is in its bundle, and Linux has no icon in the program.
//
// The translations are compiled into the program, so a change to one rebuilds it.
fn main() {
    println!("cargo:rerun-if-changed=assets/shell/app_icon.ico");
    println!("cargo:rerun-if-changed=locales");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        embed_windows_icon();
    }
}

fn embed_windows_icon() {
    winresource::WindowsResource::new()
        .set_icon("assets/shell/app_icon.ico")
        .compile()
        .expect("the icon goes into Chess.exe");
}
