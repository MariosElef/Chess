use super::app::{AppError, FieldError};

/// Checks every field of a request before answering, so one 422 names all that are wrong.
#[derive(Debug, Default)]
pub struct Fields {
    errors: Vec<FieldError>,
}

impl Fields {
    /// The checked value, or its default when refused; `finish` refuses before it is used.
    pub fn check<T: Default>(&mut self, checked: Result<T, FieldError>) -> T {
        checked.unwrap_or_else(|error| {
            self.errors.push(error);
            T::default()
        })
    }

    pub fn finish(self) -> Result<(), AppError> {
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(AppError::fields(self.errors))
        }
    }
}
