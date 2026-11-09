use serde::Serialize;

/// A new account. No `Debug`: it carries the password. The email is optional: left blank, the
/// server keeps none.
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
