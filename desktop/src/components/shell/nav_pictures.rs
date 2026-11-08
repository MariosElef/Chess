use crate::enums::shell::Page;

/// The rail's pictures: each page's asset path and its bytes.
const PICTURES: [(Page, &str, &[u8]); 4] = [
    (
        Page::Play,
        "nav/play.png",
        include_bytes!("../../../assets/nav/play.png"),
    ),
    (
        Page::Puzzles,
        "nav/puzzles.png",
        include_bytes!("../../../assets/nav/puzzles.png"),
    ),
    (
        Page::Analyze,
        "nav/analyse.png",
        include_bytes!("../../../assets/nav/analyse.png"),
    ),
    (
        Page::Social,
        "nav/social.png",
        include_bytes!("../../../assets/nav/social.png"),
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
