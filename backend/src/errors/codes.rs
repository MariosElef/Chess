use axum::http::StatusCode;

/// Every problem the server answers with. A code is for a case the client handles
/// differently; its title is the sentence a person reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChessCode {
    InvalidRequestBody,
    InvalidFields,
    RouteNotFound,
    SignInRequired,
    InvalidCredentials,
    EmailTaken,
    UsernameTaken,
    AllGuestsBusy,
    LichessUnavailable,
    NoPuzzleInRange,
    UserNotFound,
    NoFriendRequest,
    CannotBefriendYourself,
    ConversationNotFound,
    OnlyFriends,
    NotAGroup,
    MessageLength,
    GroupTitle,
    GuestCannotClose,
    InternalError,
}

impl ChessCode {
    pub fn status(self) -> StatusCode {
        match self {
            Self::InvalidRequestBody => StatusCode::BAD_REQUEST,
            Self::InvalidFields => StatusCode::UNPROCESSABLE_ENTITY,
            Self::RouteNotFound
            | Self::NoPuzzleInRange
            | Self::UserNotFound
            | Self::NoFriendRequest => StatusCode::NOT_FOUND,
            Self::CannotBefriendYourself => StatusCode::UNPROCESSABLE_ENTITY,
            Self::ConversationNotFound => StatusCode::NOT_FOUND,
            Self::OnlyFriends => StatusCode::FORBIDDEN,
            Self::NotAGroup => StatusCode::CONFLICT,
            Self::MessageLength | Self::GroupTitle => StatusCode::UNPROCESSABLE_ENTITY,
            Self::GuestCannotClose => StatusCode::FORBIDDEN,
            Self::SignInRequired | Self::InvalidCredentials => StatusCode::UNAUTHORIZED,
            Self::EmailTaken | Self::UsernameTaken | Self::AllGuestsBusy => StatusCode::CONFLICT,
            Self::LichessUnavailable => StatusCode::SERVICE_UNAVAILABLE,
            Self::InternalError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::InvalidRequestBody => "The request body could not be read",
            Self::InvalidFields => "Some fields are not valid",
            Self::RouteNotFound => "Not found",
            Self::SignInRequired => "Sign in required",
            Self::InvalidCredentials => "Wrong email, username or password",
            Self::EmailTaken => "Email already in use",
            Self::UsernameTaken => "Username already taken",
            Self::AllGuestsBusy => "Every guest account is in use: sign up to play",
            Self::LichessUnavailable => "Puzzles are unavailable right now",
            Self::NoPuzzleInRange => "No new puzzle in that range: try a wider one",
            Self::UserNotFound => "No such player",
            Self::NoFriendRequest => "No friend request from that player",
            Self::CannotBefriendYourself => "You cannot add yourself",
            Self::ConversationNotFound => "No such conversation",
            Self::OnlyFriends => "You can only chat with friends",
            Self::NotAGroup => "Only a group chat can do that",
            Self::MessageLength => "A message is 1 to 2000 characters",
            Self::GroupTitle => "A group needs a name of up to 60 characters",
            Self::GuestCannotClose => "A guest account is shared and cannot be deleted",
            Self::InternalError => "Something went wrong",
        }
    }

    /// The problem's `type`, which a client keys on and which never changes once shipped.
    pub fn problem_type(self) -> String {
        format!("/problems/{}", self.slug())
    }

    fn slug(self) -> &'static str {
        match self {
            Self::InvalidRequestBody => "invalid-request-body",
            Self::InvalidFields => "invalid-fields",
            Self::RouteNotFound => "route-not-found",
            Self::SignInRequired => "sign-in-required",
            Self::InvalidCredentials => "invalid-credentials",
            Self::EmailTaken => "email-taken",
            Self::UsernameTaken => "username-taken",
            Self::AllGuestsBusy => "all-guests-busy",
            Self::LichessUnavailable => "lichess-unavailable",
            Self::NoPuzzleInRange => "no-puzzle-in-range",
            Self::UserNotFound => "user-not-found",
            Self::NoFriendRequest => "no-friend-request",
            Self::CannotBefriendYourself => "cannot-befriend-yourself",
            Self::ConversationNotFound => "conversation-not-found",
            Self::OnlyFriends => "only-friends",
            Self::NotAGroup => "not-a-group",
            Self::MessageLength => "message-length",
            Self::GroupTitle => "group-title",
            Self::GuestCannotClose => "guest-cannot-close",
            Self::InternalError => "internal-error",
        }
    }
}
