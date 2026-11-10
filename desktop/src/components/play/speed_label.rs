use chess_domain::Category;
use gpui_kit::{component::h_flex, prelude::FluentBuilder as _, *};

use crate::services::play::category_name;

/// Bullet's lightning: also Puzzle Rush's picture.
pub const BULLET_PICTURE: &str = "speed/bullet.png";

/// The speeds' pictures: each one's asset path and its bytes.
const PICTURES: [(Category, &str, &[u8]); 4] = [
    (
        Category::Bullet,
        BULLET_PICTURE,
        include_bytes!("../../../assets/speed/bullet.png"),
    ),
    (
        Category::Blitz,
        "speed/blitz.png",
        include_bytes!("../../../assets/speed/blitz.png"),
    ),
    (
        Category::Rapid,
        "speed/rapid.png",
        include_bytes!("../../../assets/speed/rapid.png"),
    ),
    (
        Category::Classical,
        "speed/classical.svg",
        include_bytes!("../../../assets/speed/classical.svg"),
    ),
];

/// A speed's name with its picture before it, where it has one.
pub fn speed_label(category: Category) -> Div {
    h_flex()
        .gap_1p5()
        .text_sm()
        .when_some(speed_picture(category), |this, path| {
            this.child(img(path).size_4().flex_shrink_0())
        })
        .child(category_name(category))
}

/// A speed's picture, by asset path, where it has one.
pub fn speed_picture(category: Category) -> Option<&'static str> {
    PICTURES
        .iter()
        .find(|(pictured, _, _)| *pictured == category)
        .map(|(_, path, _)| *path)
}

/// The picture at `path`, if it is one of the speeds'.
pub fn speed_picture_bytes(path: &str) -> Option<&'static [u8]> {
    PICTURES
        .iter()
        .find(|(_, pictured, _)| *pictured == path)
        .map(|(_, _, bytes)| *bytes)
}

/// Every speed picture's path, for listing.
pub fn speed_picture_paths() -> impl Iterator<Item = &'static str> {
    PICTURES.iter().map(|(_, path, _)| *path)
}
