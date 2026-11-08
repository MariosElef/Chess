/// A computer opponent: who it is, how strong it plays, and its picture, if it has one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bot {
    pub name: &'static str,
    pub rating: u32,
    /// The picture's asset path.
    pub picture: Option<&'static str>,
}
