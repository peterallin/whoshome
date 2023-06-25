use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait Router {
    async fn known_clients(&self) -> Result<Vec<Client>>;
    async fn online_clients(&self) -> Result<Vec<Client>>;
    async fn block_client(&self, client: &Client) -> Result<()>;
    async fn unblock_client(&self, client: &Client) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct Client {
    pub name: String,
    pub mac: String,
}
