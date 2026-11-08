use crate::{
    errors::AppError,
    repositories::database::{self, Db},
};

/// Whether the server can do its work: today, whether the database answers.
#[derive(Clone)]
pub struct HealthService {
    db: Db,
}

impl HealthService {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    pub async fn ready(&self) -> Result<(), AppError> {
        database::ping(&self.db).await?;
        Ok(())
    }
}
