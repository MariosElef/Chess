/// What an account form shows after a refusal: a sentence per field at fault, and one overall.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FormProblems {
    pub login: Option<String>,
    pub email: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub form: Option<String>,
}
