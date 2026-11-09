/// Where a picture is asked for: the app's own asset path, which the asset loader fetches.
const PREFIX: &str = "avatars/";

/// The asset path of the player's picture, if they have one.
pub fn avatar_path(user_id: &str, hash: Option<&str>) -> Option<String> {
    hash.map(|hash| format!("{PREFIX}{user_id}/{hash}"))
}

/// The player and picture hash an asset path names, if it names a picture.
pub fn avatar_of_path(path: &str) -> Option<(&str, &str)> {
    path.strip_prefix(PREFIX)?.split_once('/')
}

#[cfg(test)]
mod tests {
    use super::{avatar_of_path, avatar_path};

    #[test]
    fn a_picture_path_names_its_player_and_hash() {
        let path = avatar_path("u1", Some("abc")).expect("a path");
        assert_eq!(avatar_of_path(&path), Some(("u1", "abc")));
        assert_eq!(avatar_path("u1", None), None);
        assert_eq!(avatar_of_path("pieces/classic/wK.png"), None);
    }
}
