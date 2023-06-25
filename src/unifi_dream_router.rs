use std::{ops::DerefMut, sync::Arc};

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use early::Early;
use netrc_rs::Netrc;
use reqwest::{header::HeaderMap, RequestBuilder, Response, StatusCode};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tracing::{debug, info, trace};

use crate::router::Client;

pub struct UnifiDreamRouter {
    http_client: reqwest::Client,
    login_url: String,
    known_devices_url: String,
    connected_devices_url: String,
    site_url: String,
    hostname: String,
    csrf_token: Arc<Mutex<Option<String>>>,
}

#[async_trait]
impl crate::router::Router for UnifiDreamRouter {
    async fn known_clients(&self) -> Result<Vec<Client>> {
        info!(
            "Getting list of known clients from UnifiDreamRouter: {}",
            self.hostname
        );
        self.get_client_list(&self.known_devices_url).await
    }

    async fn online_clients(&self) -> Result<Vec<Client>> {
        info!(
            "Getting list of connected clients from UnifiDreamRouter: {}",
            self.hostname
        );
        self.get_client_list(dbg!(&self.connected_devices_url))
            .await
    }

    async fn block_client(&self, client: &Client) -> Result<()> {
        info!("Blocking {}", client.name);
        let cmd_url = format!("{}/cmd/stamgr", self.site_url);
        let req = self
            .http_client
            .post(cmd_url)
            .json(&BlockCommand::new(&client.mac));
        self.send(req).await?;
        Ok(())
    }

    async fn unblock_client(&self, client: &Client) -> Result<()> {
        info!("Unblocking {}", client.name);
        let cmd_url = format!("{}/cmd/stamgr", self.site_url);
        let req = self
            .http_client
            .post(cmd_url)
            .json(&UnblockCommand::new(&client.mac));
        self.send(req).await?;
        Ok(())
    }
}

impl UnifiDreamRouter {
    pub fn new(hostname: &str) -> Result<Self> {
        let router = Early::new("https", hostname);
        let login_url = router
            .clone()
            .path("api")
            .path("auth")
            .path("login")
            .build();
        let api = router.path("proxy").path("network").path("api");
        let site = api.path("s").path("default");
        let known_devices_url = site.clone().path("rest").path("user").build();
        let connected_devices_url = site.clone().path("stat").path("sta").build();
        let http_client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .cookie_store(true)
            .build()
            .context("Failed to build http client")?;

        Ok(UnifiDreamRouter {
            login_url,
            known_devices_url,
            connected_devices_url,
            http_client,
            site_url: site.build(),
            hostname: hostname.to_owned(),
            csrf_token: Arc::new(Mutex::new(None)),
        })
    }

    async fn send(&self, request: RequestBuilder) -> Result<Response> {
        let request = self.add_csrf_header(request).await;
        let backup = request
            .try_clone()
            .ok_or_else(|| anyhow!("Failed to clone request"))?;

        let response = match request.send().await?.error_for_status() {
            Ok(response) => response,
            Err(e) => {

                if e.status()
                    .ok_or_else(|| anyhow!("Failed to get status from response"))?
                    == StatusCode::UNAUTHORIZED
                {
                    trace!("Got 401, authenticating on: {}", self.hostname);
                    self.login().await.context("Failed to login on router")?;
                    trace!("Authenticating finished sending request again");
                    backup.send().await?.error_for_status()?
                } else {
                    return Err(e.into());
                }
            }
        };

        let token = get_csrf_token(response.headers())?;
        debug!("Got CSRF token: {:?}", token);
        *self.csrf_token.lock().await = token;

        Ok(response)
    }

    async fn add_csrf_header(&self, request: RequestBuilder) -> RequestBuilder {
        let mut csrf_token = self.csrf_token.lock().await;
        match csrf_token.deref_mut() {
            Some(t) => {
                debug!("Adding csrf token: {}", t);
                let req = request.header("x-csrf-token", t.clone());
                *csrf_token = Some(t.to_owned());
                req
            }
            None => request,
        }
    }

    async fn login(&self) -> Result<()> {
        let password = get_password(&self.hostname)
            .with_context(|| format!("Failed to get password for {}", self.hostname))?;
        let resp = self
            .http_client
            .post(&self.login_url)
            .json(&Login {
                username: "peter@peca.dk".into(),
                password,
            })
            .send()
            .await
            .context("Login to router failed")?
            .error_for_status()
            .context("Login failed")?;
        let token = get_csrf_token(resp.headers())?;
        debug!("Got CSRF token at login: {:?}", token);
        *self.csrf_token.lock().await = token;
        Ok(())
    }

    async fn get_client_list(&self, url: &str) -> Result<Vec<Client>> {
        let request = self.http_client.get(url);
        let resp = self.send(request).await;
        let client_devices: RouterResponse<UnifiClient> = resp?.error_for_status()?.json().await?;
        let client_devices = client_devices.data;

        Ok(client_devices
            .into_iter()
            .map(|c| Client {
                name: c.name(),
                mac: c.mac,
            })
            .collect())
    }
}

fn get_csrf_token(headers: &HeaderMap) -> Result<Option<String>> {
    let header = match headers.get("x-csrf-token") {
        Some(h) => h,
        None => return Ok(None),
    };
    let csrf_token = header
        .to_str()
        .context("Failed to get string from CSRF token header")?;
    Ok(Some(csrf_token.to_owned()))
}

#[derive(Deserialize, Debug)]
struct UnifiClient {
    name: Option<String>,
    mac: String,
}

impl UnifiClient {
    fn name(&self) -> String {
        self.name
            .clone()
            .unwrap_or_else(|| "<unnamed client>".to_string())
    }
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

#[derive(Serialize)]
struct BlockCommand {
    cmd: String,
    mac: String,
}

impl BlockCommand {
    fn new(mac: &str) -> Self {
        Self {
            cmd: "block-sta".into(),
            mac: mac.into(),
        }
    }
}

#[derive(Serialize)]
struct UnblockCommand {
    cmd: String,
    mac: String,
}

impl UnblockCommand {
    fn new(mac: &str) -> Self {
        Self {
            cmd: "unblock-sta".into(),
            mac: mac.into(),
        }
    }
}
