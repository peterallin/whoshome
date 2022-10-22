use anyhow::Result;

pub trait Router {
    fn connected_clients(&self) -> Result<Vec<String>>;
}
