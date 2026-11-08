use crate::enums::shell::Page;

/// The Play page's picture: on the rail and on its panels' title strips.
pub const PLAY_PICTURE: &str = "nav/play.png";
/// The Puzzles page's picture: on the rail, its menus and the account's puzzle rating.
pub const PUZZLES_PICTURE: &str = "nav/puzzles.png";
/// The Social page's picture: on the rail and over the Friends page.
pub const SOCIAL_PICTURE: &str = "nav/social.png";
/// The Messages page's picture: in Social's menu on the rail and over the page.
pub const MESSAGES_PICTURE: &str = "nav/messages.png";
/// The Analysis page's picture: on the rail and on its panel's title strip.
pub const ANALYSIS_PICTURE: &str = "nav/analyse.png";

/// The pages' pictures, on the rail and in its menus: each page's asset path and its bytes.
const PICTURES: [(Page, &str, &[u8]); 5] = [
    (
        Page::Play,
        PLAY_PICTURE,
        include_bytes!("../../../assets/nav/play.png"),
    ),
    (
        Page::Puzzles,
        PUZZLES_PICTURE,
        include_bytes!("../../../assets/nav/puzzles.png"),
    ),
    (
        Page::Analyze,
        ANALYSIS_PICTURE,
        include_bytes!("../../../assets/nav/analyse.png"),
    ),
    (
        Page::Social,
        SOCIAL_PICTURE,
        include_bytes!("../../../assets/nav/social.png"),
    ),
    (
        Page::Messages,
        MESSAGES_PICTURE,
        include_bytes!("../../../assets/nav/messages.png"),
    ),
];

/// The page's rail picture, by asset path; pages opened from the avatar menu have none.
pub fn page_picture(page: Page) -> Option<&'static str> {
    PICTURES
        .iter()
        .find(|(pictured, _, _)| *pictured == page)
        .map(|(_, path, _)| *path)
}

/// The picture at `path`, if it is one of the rail's.
pub fn nav_picture(path: &str) -> Option<&'static [u8]> {
    PICTURES
        .iter()
        .find(|(_, pictured, _)| *pictured == path)
        .map(|(_, _, bytes)| *bytes)
}

/// Every rail picture's path, for listing.
pub fn nav_picture_paths() -> impl Iterator<Item = &'static str> {
    PICTURES.iter().map(|(_, path, _)| *path)
}
