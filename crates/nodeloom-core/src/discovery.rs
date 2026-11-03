//! mDNS advertisement (master) and lookup (agent).

use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::Duration,
};

use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use nodeloom_proto::MDNS_SERVICE_TYPE;
use tracing::info;

use crate::netif;

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

/// After the first answer, how long to keep listening for the master's
/// other addresses (one per network it is on).
const GATHER: Duration = Duration::from_millis(1500);
/// How long each address gets to accept a connection.
const PROBE: Duration = Duration::from_millis(800);
/// Addresses tried at most; a master is on a handful of networks.
const MAX_TRIED: usize = 6;

/// Waits up to `timeout` for a master to announce itself and returns the
/// address to reach it at. A master on several networks (Wi-Fi and a cable)
/// announces several addresses: the best path from this machine that
/// accepts a connection wins (see [`netif::rank`]).
pub async fn find_master(timeout: Duration) -> Result<(IpAddr, u16), DiscoveryError> {
    let (addresses, port) = gather(timeout).await?;
    let ranked = netif::rank(&addresses, &netif::local_ipv4());
    let tried: Vec<SocketAddr> = ranked
        .iter()
        .take(MAX_TRIED)
        .map(|ip| SocketAddr::from((*ip, port)))
        .collect();
    let chosen = netif::first_reachable(&tried, PROBE)
        .await
        // None answered (a firewall, a slow network): the best guess still
        // gets the agent's usual retries.
        .unwrap_or(tried[0]);
    if ranked.len() > 1 {
        info!("master announced {ranked:?}; using {}", chosen.ip());
    }
    Ok((chosen.ip(), port))
}

/// Every IPv4 address the first master found announces, and its port.
async fn gather(timeout: Duration) -> Result<(Vec<Ipv4Addr>, u16), DiscoveryError> {
    let daemon = ServiceDaemon::new()?;
    let receiver = daemon.browse(MDNS_SERVICE_TYPE)?;
    let mut deadline = tokio::time::Instant::now() + timeout;
    // (instance name, port, addresses) of the first master that answered.
    let mut found: Option<(String, u16, Vec<Ipv4Addr>)> = None;

    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        match tokio::time::timeout(remaining, receiver.recv_async()).await {
            Ok(Ok(ServiceEvent::ServiceResolved(info))) => {
                let name = info.get_fullname().to_owned();
                let addrs: Vec<Ipv4Addr> = info.get_addresses_v4().into_iter().collect();
                match &mut found {
                    None if !addrs.is_empty() => {
                        found = Some((name, info.get_port(), addrs));
                        deadline = deadline.min(tokio::time::Instant::now() + GATHER);
                    }
                    Some((first, _, known)) if *first == name => {
                        for a in addrs {
                            if !known.contains(&a) {
                                known.push(a);
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Ok(_)) => {}
            Ok(Err(_)) | Err(_) => break,
        }
    }
    let _ = daemon.shutdown();
    found
        .map(|(_, port, addrs)| (addrs, port))
        .ok_or(DiscoveryError::NotFound(timeout))
}
