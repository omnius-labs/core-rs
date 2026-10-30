use std::fmt;

use testcontainers::{ContainerAsync, GenericImage};

use crate::prelude::*;

/// The address a container port is reachable at from the test process.
#[derive(Clone, Debug)]
pub struct Endpoint {
    pub host: url::Host,
    pub port: u16,
}

impl Endpoint {
    pub async fn new(container: &ContainerAsync<GenericImage>, port: u16) -> Result<Self> {
        let host = match std::env::var("TESTCONTAINERS_HOST_OVERRIDE") {
            Ok(value) if !value.trim().is_empty() => Self::parse_host(value.trim())?,
            _ => container.get_host().await?,
        };
        let port = container.get_host_port_ipv4(port).await?;

        Ok(Self { host, port })
    }

    fn parse_host(value: &str) -> Result<url::Host> {
        if let Ok(address) = value.parse::<std::net::Ipv6Addr>() {
            return Ok(url::Host::Ipv6(address));
        }

        Ok(url::Host::parse(value)?)
    }
}

impl fmt::Display for Endpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.host, self.port)
    }
}
