use serde::Serialize;

/// A new account. No `Debug`: it carries the password.
#[derive(Clone, Serialize)]
pub struct SignUp {
    pub email: String,
    pub username: String,
    pub password: String,
}

/// A sign-in by email or username. No `Debug`: it carries the password.
#[derive(Clone, Serialize)]
pub struct SignIn {
    pub login: String,
    pub password: String,
}
