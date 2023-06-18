use anyhow::Result;

pub trait Router {
    fn known_clients(&self) -> Result<Vec<Client>>;
    fn block_client(&self, client: &Client) -> Result<()>;
    fn unblock_client(&self, client: &Client) -> Result<()>;
}

#[derive(Debug)]
pub struct Client {
    pub name: String,
    pub mac: String,
}
