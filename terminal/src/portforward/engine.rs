#![cfg(feature = "server")]

use std::collections::HashMap;
use std::net::ToSocketAddrs;

use nameth::NamedEnumValues as _;
use nameth::nameth;
use tokio::net::TcpListener;

use super::listeners::listeners;
use super::schema::PortForward;

pub async fn process(old: &[PortForward], new: &[PortForward]) -> Result<(), PortForwardError> {
    let old = old
        .iter()
        .map(|old| (old.id, old))
        .collect::<HashMap<_, _>>();
    for new in new {
        let () = process_port_forward(old.get(&new.id).copied(), new).await?;
    }
    Ok(())
}

async fn process_port_forward(
    old: Option<&PortForward>,
    new: &PortForward,
) -> Result<(), PortForwardError> {
    if old == Some(new) {
        return Ok(());
    }
    let from = new.from.clone();
    let endpoint = format!("{}:{}", from.host, from.port);
    let addresses = endpoint
        .to_socket_addrs()
        .map_err(PortForwardError::Hostname)?;
    let mut listeners = listeners();
    let mut listeners = listeners.entry(new.id).insert_entry(HashMap::default());
    for address in addresses {
        let listener = TcpListener::bind(address)
            .await
            .map_err(PortForwardError::Bind)?;
        listeners.get_mut().insert(address, listener);
    }
    Ok(())
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum PortForwardError {
    #[error("[{n}] Failed to resolve: {0}", n = self.name())]
    Hostname(std::io::Error),

    #[error("[{n}] Failed to bind: {0}", n = self.name())]
    Bind(std::io::Error),
}

impl From<PortForwardError> for tonic::Status {
    fn from(error: PortForwardError) -> Self {
        let code = match &error {
            PortForwardError::Hostname { .. } => tonic::Code::InvalidArgument,
            PortForwardError::Bind { .. } => tonic::Code::InvalidArgument,
        };
        Self::new(code, error.to_string())
    }
}
