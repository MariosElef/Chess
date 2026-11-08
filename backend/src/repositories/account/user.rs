use sqlx::PgExecutor;
use uuid::Uuid;

use crate::models::account::{GetUserModel, NewUser, UserCredentials};

pub async fn email_taken(db: impl PgExecutor<'_>, email: &str) -> Result<bool, sqlx::Error> {
    sqlx::query_file_scalar!("src/repositories/sql/account/user/email_taken.sql", email)
        .fetch_one(db)
        .await
}

pub async fn username_taken(db: impl PgExecutor<'_>, username: &str) -> Result<bool, sqlx::Error> {
    sqlx::query_file_scalar!(
        "src/repositories/sql/account/user/username_taken.sql",
        username
    )
    .fetch_one(db)
    .await
}

/// Whether the account went in; `false` when its email or username was taken meanwhile.
pub async fn insert(db: impl PgExecutor<'_>, user: &NewUser) -> Result<bool, sqlx::Error> {
    let result = sqlx::query_file!(
        "src/repositories/sql/account/user/insert.sql",
        user.id,
        user.email,
        user.username,
        user.password_hash,
    )
    .execute(db)
    .await?;
    Ok(result.rows_affected() == 1)
}

/// The open account whose email or username is `login`.
pub async fn find_credentials(
    db: impl PgExecutor<'_>,
    login: &str,
) -> Result<Option<UserCredentials>, sqlx::Error> {
    sqlx::query_file_as!(
        UserCredentials,
        "src/repositories/sql/account/user/find_credentials.sql",
        login,
    )
    .fetch_optional(db)
    .await
}

/// The first guest account nobody is signed in as, locked for the caller's transaction.
pub async fn claim_free_guest(
    db: impl PgExecutor<'_>,
    in_use: &[Uuid],
) -> Result<Option<GetUserModel>, sqlx::Error> {
    sqlx::query_file_as!(
        GetUserModel,
        "src/repositories/sql/account/user/claim_free_guest.sql",
        in_use
    )
    .fetch_optional(db)
    .await
}

pub async fn get(
    db: impl PgExecutor<'_>,
    user_id: Uuid,
) -> Result<Option<GetUserModel>, sqlx::Error> {
    sqlx::query_file_as!(
        GetUserModel,
        "src/repositories/sql/account/user/get.sql",
        user_id
    )
    .fetch_optional(db)
    .await
}
