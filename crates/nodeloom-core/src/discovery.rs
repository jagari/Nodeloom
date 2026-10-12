//! mDNS advertisement (master) and lookup (agent).

use std::{net::IpAddr, time::Duration};

use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use nodeloom_proto::MDNS_SERVICE_TYPE;

#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
    #[error("mdns: {0}")]
    Mdns(#[from] mdns_sd::Error),
    #[error("no master found within {0:?}")]
    NotFound(Duration),
}

/// Keeps the master's advertisement alive; it is withdrawn on drop.
pub struct Advertisement {
    daemon: ServiceDaemon,
}

impl Drop for Advertisement {
    fn drop(&mut self) {
        let _ = self.daemon.shutdown();
    }
}

/// Advertises a master listening on `port` under the name `instance`.
pub fn advertise(
    instance: &str,
    hostname: &str,
    port: u16,
) -> Result<Advertisement, DiscoveryError> {
    let daemon = ServiceDaemon::new()?;
    let host = format!("{}.local.", hostname.trim_end_matches('.'));
    let info = ServiceInfo::new(
        MDNS_SERVICE_TYPE,
        instance,
        &host,
        "",
        port,
        None::<std::collections::HashMap<String, String>>,
    )?
    .enable_addr_auto();
    daemon.register(info)?;
    Ok(Advertisement { daemon })
}

/// Waits up to `timeout` for a master to announce itself and returns its
/// address and port.
pub async fn find_master(timeout: Duration) -> Result<(IpAddr, u16), DiscoveryError> {
    let daemon = ServiceDaemon::new()?;
    let receiver = daemon.browse(MDNS_SERVICE_TYPE)?;
    let deadline = tokio::time::Instant::now() + timeout;

    let found = loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        match tokio::time::timeout(remaining, receiver.recv_async()).await {
            Ok(Ok(ServiceEvent::ServiceResolved(info))) => {
                let addr = info.get_addresses_v4().into_iter().next().map(IpAddr::V4);
                if let Some(addr) = addr {
                    break Some((addr, info.get_port()));
                }
            }
            Ok(Ok(_)) => {}
            Ok(Err(_)) | Err(_) => break None,
        }
    };
    let _ = daemon.shutdown();
    found.ok_or(DiscoveryError::NotFound(timeout))
}
