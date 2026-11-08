use gpui_kit::assets::IconName;

use crate::enums::shell::Page;

/// Each page's icon, for the avatar menu's Account and Settings.
pub fn page_icon(page: Page) -> IconName {
    match page {
        Page::Play => IconName::ChessKnight,
        Page::Puzzles => IconName::Puzzle,
        Page::Analyze => IconName::Microscope,
        Page::Social => IconName::Users,
        Page::Messages => IconName::Mail,
        Page::Settings => IconName::Settings,
        Page::Account => IconName::CircleUser,
    }
}
