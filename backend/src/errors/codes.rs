use axum::http::StatusCode;

/// Every problem the server answers with: a code the client acts on and a title a person reads.
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
    GameNotFound,
    OnlyFriends,
    NotAGroup,
    NotGroupOwner,
    CannotJoinGroup,
    MessageLength,
    GroupTitle,
    GuestCannotClose,
    GuestCannotHaveAvatar,
    AvatarUnreadable,
    AvatarTooLarge,
    AvatarNotFound,
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
            Self::ConversationNotFound | Self::GameNotFound => StatusCode::NOT_FOUND,
            Self::OnlyFriends => StatusCode::FORBIDDEN,
            Self::NotAGroup => StatusCode::CONFLICT,
            Self::NotGroupOwner | Self::CannotJoinGroup => StatusCode::FORBIDDEN,
            Self::MessageLength | Self::GroupTitle => StatusCode::UNPROCESSABLE_ENTITY,
            Self::GuestCannotClose | Self::GuestCannotHaveAvatar => StatusCode::FORBIDDEN,
            Self::AvatarUnreadable | Self::AvatarTooLarge => StatusCode::UNPROCESSABLE_ENTITY,
            Self::AvatarNotFound => StatusCode::NOT_FOUND,
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
            Self::GameNotFound => "No such game of yours",
            Self::OnlyFriends => "You can only chat with friends",
            Self::NotAGroup => "Only a group chat can do that",
            Self::NotGroupOwner => "Only the group's owner can do that",
            Self::CannotJoinGroup => "That player cannot be in this group",
            Self::MessageLength => "A message is 1 to 2000 characters",
            Self::GroupTitle => "A group needs a name of up to 60 characters",
            Self::GuestCannotClose => "A guest account is shared and cannot be deleted",
            Self::GuestCannotHaveAvatar => "A guest account is shared and cannot have a picture",
            Self::AvatarUnreadable => "That file is not a picture this server can read",
            Self::AvatarTooLarge => "That picture is too large: pick a smaller one",
            Self::AvatarNotFound => "No such picture",
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
            Self::GameNotFound => "game-not-found",
            Self::OnlyFriends => "only-friends",
            Self::NotAGroup => "not-a-group",
            Self::NotGroupOwner => "not-group-owner",
            Self::CannotJoinGroup => "cannot-join-group",
            Self::MessageLength => "message-length",
            Self::GroupTitle => "group-title",
            Self::GuestCannotClose => "guest-cannot-close",
            Self::GuestCannotHaveAvatar => "guest-cannot-have-avatar",
            Self::AvatarUnreadable => "avatar-unreadable",
            Self::AvatarTooLarge => "avatar-too-large",
            Self::AvatarNotFound => "avatar-not-found",
            Self::InternalError => "internal-error",
        }
    }
}
