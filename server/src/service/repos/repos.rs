use crate::service::domain::Service;

use std::error::Error;

#[async_trait::async_trait]
pub trait IServiceRepo: Send + Sync {
    async fn insert(&self, service: &Service) -> Result<(), Box<dyn Error>>;
    async fn save(&self, service: &Service) -> Result<(), Box<dyn Error>>;
    async fn find(&self, service_id: &str) -> Option<Service>;
    async fn delete(&self, service_id: &str) -> Option<Service>;
}