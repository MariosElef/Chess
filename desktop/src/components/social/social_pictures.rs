/// The crossed swords on a friend's Challenge button.
pub const CHALLENGE_PICTURE: &str = "social/challenge.svg";

/// Social's own pictures: each one's asset path and its bytes.
const PICTURES: [(&str, &[u8]); 1] = [(
    CHALLENGE_PICTURE,
    include_bytes!("../../../assets/social/challenge.svg"),
)];

/// The picture at `path`, if it is one of Social's.
pub fn social_picture(path: &str) -> Option<&'static [u8]> {
    PICTURES
        .iter()
        .find(|(pictured, _)| *pictured == path)
        .map(|(_, bytes)| *bytes)
}

/// Every Social picture's path, for listing.
pub fn social_picture_paths() -> impl Iterator<Item = &'static str> {
    PICTURES.iter().map(|(path, _)| *path)
}
