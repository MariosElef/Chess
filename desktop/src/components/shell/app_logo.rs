/// The app's logo, a green pawn, at the left of the header.
pub const APP_LOGO: &str = "shell/logo.svg";

const APP_LOGO_BYTES: &[u8] = include_bytes!("../../../assets/shell/logo.svg");

/// The app's icon for the Dock: the pawn on a dark rounded square, 1024 pixels across.
pub const APP_ICON_PNG: &[u8] = include_bytes!("../../../assets/shell/app_icon.png");

/// The logo's bytes, if `path` names it.
pub fn app_logo(path: &str) -> Option<&'static [u8]> {
    (path == APP_LOGO).then_some(APP_LOGO_BYTES)
}
