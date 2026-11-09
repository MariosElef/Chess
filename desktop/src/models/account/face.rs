/// How a person shows beside their name: their picture's asset path, or their initial without one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Face {
    pub name: String,
    pub picture: Option<String>,
}
