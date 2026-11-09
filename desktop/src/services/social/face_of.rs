use super::avatar_path;
use crate::models::account::Face;

/// How a person shows beside their name: their picture, or their initial without one.
pub fn face_of(user_id: &str, username: &str, hash: Option<&str>) -> Face {
    Face {
        name: username.to_owned(),
        picture: avatar_path(user_id, hash),
    }
}
