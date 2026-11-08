use gpui_kit::{
    App, Window,
    component::{Theme, ThemeMode},
};

use crate::enums::settings::Appearance;

/// Puts the window in the chosen appearance; System follows the Mac's own setting.
pub fn apply_appearance(appearance: Appearance, window: &mut Window, cx: &mut App) {
    match appearance {
        Appearance::System => Theme::sync_system_appearance(Some(window), cx),
        Appearance::Light => Theme::change(ThemeMode::Light, Some(window), cx),
        Appearance::Dark => Theme::change(ThemeMode::Dark, Some(window), cx),
    }
}
