use crate::{clients::files::PictureFileError, models::account::AccountProblem};

/// What the Account page says when a picture file cannot be sent.
pub fn picture_file_problem(error: &PictureFileError) -> AccountProblem {
    match error {
        PictureFileError::TooLarge => AccountProblem::PictureTooLarge,
        PictureFileError::Unreadable => AccountProblem::PictureUnreadable,
    }
}
