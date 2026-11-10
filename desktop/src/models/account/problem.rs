use crate::models::server::ServerError;

/// What went wrong on an account screen, kept as a cause rather than as words so it is worded
/// in the language being shown when it is drawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountProblem {
    /// The server refused, could not be reached, or answered something unreadable.
    Server(ServerError),
    /// The session ended on another device.
    SignedOut,
    /// The email field is not an address.
    InvalidEmail,
    /// The username field is the wrong length or has characters a username cannot.
    UsernameFormat,
    /// The password field is the wrong length.
    PasswordLength,
    PictureTooLarge,
    PictureUnreadable,
    /// A game of the history could not be read back to open it.
    GameUnreadable,
}
