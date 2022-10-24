use anyhow::{anyhow, Context, Result};
use early::Early;
use netrc_rs::Netrc;
use reqwest::{
    blocking::{RequestBuilder, Response},
    StatusCode,
};
use serde::{Deserialize, Serialize};
use tracing::{info, trace};

#[derive(Debug)]
pub struct UnifiDreamRouter {
    http_client: reqwest::blocking::Client,
    login_url: String,
    client_devices_url: String,
    hostname: String,
}

impl crate::router::Router for UnifiDreamRouter {
    fn connected_clients(&self) -> Result<Vec<String>> {
        info!("Getting list of connected clients from UnifiDreamRouter: {}", self.hostname);
        let request = self.http_client.get(&self.client_devices_url);
        self.send(request)?;

        let client_devices: RouterResponse<Client> = self
            .http_client
            .get(&self.client_devices_url)
            .send()?
            .error_for_status()?
            .json()?;
        let client_devices = client_devices.data;

        Ok(client_devices.into_iter().filter_map(|c| c.name).collect())
    }
}

impl UnifiDreamRouter {
    pub fn new(hostname: &str) -> Result<Self> {
        let router = Early::new("https", "router");
        let login_url = router
            .clone()
            .path("api")
            .path("auth")
            .path("login")
            .build();
        let api = router.path("proxy").path("network").path("api");
        let site = api.path("s").path("default");
        let stat = site.path("stat");
        let client_devices_url = stat.path("sta").build();
        let http_client = reqwest::blocking::Client::builder()
            .danger_accept_invalid_certs(true)
            .cookie_store(true)
            .build()
            .context("Failed to build http client")?;

        Ok(UnifiDreamRouter {
            login_url,
            client_devices_url,
            http_client,
            hostname: hostname.to_owned(),
        })
    }

    fn send(&self, request: RequestBuilder) -> Result<Response> {
        let backup = request
            .try_clone()
            .ok_or_else(|| anyhow!("Failed to clone request"))?;
        let response = match request.send()?.error_for_status() {
            Ok(response) => response,
            Err(e) => {
                if e.status()
                    .ok_or_else(|| anyhow!("Failed to get status from response"))?
                    == StatusCode::UNAUTHORIZED
                {
                    trace!("Got 401, authenticating on: {}", self.hostname);
                    self.login().context("Failed to login on router")?;
                    trace!("Authorizing finished sending request again");
                    backup.send()?.error_for_status()?
                } else {
                    return Err(e.into());
                }
            }
        };
        Ok(response)
    }

    fn login(&self) -> Result<()> {
        let password = get_password(&self.hostname)
            .with_context(|| format!("Failed to get password for {}", self.hostname))?;
        self.http_client
            .post(&self.login_url)
            .json(&Login {
                username: "peter@peca.dk".into(),
                password,
            })
            .send()
            .context("Login to router failed")?
            .error_for_status()
            .context("Login failed")?;

        Ok(())
    }
}

#[derive(Deserialize, Debug)]
struct Client {
    name: Option<String>,
}

#[derive(Serialize)]
struct Login {
    username: String,
    password: String,
}

#[derive(Deserialize, Debug)]
struct RouterResponse<T> {
    data: Vec<T>,
}

pub fn get_password(machine: &str) -> Result<String> {
    let home = home::home_dir().ok_or_else(|| anyhow!("Unable to find home dir"))?;
    let netrc = std::fs::read_to_string(home.join(".netrc")).context("Unable to read .netrc")?;
    let netrc = Netrc::parse(netrc, false).map_err(|e| anyhow!("unable to parse .netrc: {e}"))?;
    let password = netrc
        .machines
        .into_iter()
        .find(|m| m.name == Some(machine.into()))
        .ok_or_else(|| anyhow!("Could not find {machine} in .netrc"))?
        .password
        .ok_or_else(|| anyhow!("No password for {machine} in .netrc"))?;
    Ok(password)
}
