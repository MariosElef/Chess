use serde::Deserialize;

/// The server's answer to a refused request: RFC 9457 problem details.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Problem {
    /// What the client keys on, such as `/problems/email-taken`.
    #[serde(rename = "type")]
    pub problem_type: String,
    pub title: String,
    #[serde(default)]
    pub detail: Option<String>,
    #[serde(default)]
    pub errors: Vec<FieldProblem>,
}

/// One request field at fault, named by its JSON pointer such as `/email`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct FieldProblem {
    pub pointer: String,
    pub detail: String,
}

impl Problem {
    /// The sentence a person reads: this occurrence's detail, or else the title.
    pub fn message(&self) -> &str {
        self.detail.as_deref().unwrap_or(&self.title)
    }
}
