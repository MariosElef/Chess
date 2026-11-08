/// What an account form shows after the server refused it: a sentence under each field at
/// fault, and one for the form as a whole.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FormProblems {
    pub login: Option<String>,
    pub email: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub form: Option<String>,
}
