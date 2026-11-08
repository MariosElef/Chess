use crate::enums::analysis::MoveClass;

/// The review's icons, with the bytes each class's icon path serves.
const ICONS: [(MoveClass, &[u8]); 11] = [
    (
        MoveClass::Brilliant,
        include_bytes!("../../../assets/review/brilliant.png"),
    ),
    (
        MoveClass::Great,
        include_bytes!("../../../assets/review/great.png"),
    ),
    (
        MoveClass::Best,
        include_bytes!("../../../assets/review/best.png"),
    ),
    (
        MoveClass::Excellent,
        include_bytes!("../../../assets/review/excellent.png"),
    ),
    (
        MoveClass::Good,
        include_bytes!("../../../assets/review/good.png"),
    ),
    (
        MoveClass::Book,
        include_bytes!("../../../assets/review/book.png"),
    ),
    (
        MoveClass::Forced,
        include_bytes!("../../../assets/review/forced.png"),
    ),
    (
        MoveClass::Inaccuracy,
        include_bytes!("../../../assets/review/inaccuracy.png"),
    ),
    (
        MoveClass::Mistake,
        include_bytes!("../../../assets/review/mistake.png"),
    ),
    (
        MoveClass::Miss,
        include_bytes!("../../../assets/review/miss.png"),
    ),
    (
        MoveClass::Blunder,
        include_bytes!("../../../assets/review/blunder.png"),
    ),
];

/// The icon at `path`, if it is one of the review's.
pub fn review_icon(path: &str) -> Option<&'static [u8]> {
    ICONS
        .iter()
        .find(|(class, _)| class.icon_path() == path)
        .map(|(_, bytes)| *bytes)
}

/// Every review icon's path, for listing.
pub fn review_icon_paths() -> impl Iterator<Item = &'static str> {
    ICONS.iter().map(|(class, _)| class.icon_path())
}
