use crate::clients::files::PictureFileError;

/// What the Account page says when a picture file cannot be sent.
pub fn picture_file_problem(error: &PictureFileError) -> &'static str {
    match error {
        PictureFileError::TooLarge => "Pick a picture under 5 MB",
        PictureFileError::Unreadable => "That file could not be read",
    }
}
