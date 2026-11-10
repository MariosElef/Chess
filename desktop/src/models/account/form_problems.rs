use super::problem::AccountProblem;

/// What an account form shows after a refusal: a problem per field at fault, and one overall.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FormProblems {
    pub login: Option<AccountProblem>,
    pub email: Option<AccountProblem>,
    pub username: Option<AccountProblem>,
    pub password: Option<AccountProblem>,
    pub form: Option<AccountProblem>,
}
