//! This machine's network interfaces, and which of a master's addresses to
//! try first. A cable between two machines (a LAN cable straight between
//! them, a Thunderbolt bridge, USB4 networking) shows up as its own
//! interface, usually with a `169.254.x.x` link-local address; Wi-Fi is the
//! last resort when a cable is there too.

use std::{
    net::{Ipv4Addr, SocketAddr},
    time::Duration,
};

use serde::{Deserialize, Serialize};

/// What kind of link an interface is, as far as its name and address tell.
/// Names are a heuristic: on a Mac `en0` is Wi-Fi on a laptop but Ethernet
/// on a desktop, so such names stay `Other`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkKind {
    /// A link-local address: a cable straight to another machine.
    Direct,
    Thunderbolt,
    Wired,
    Other,
    #[serde(rename = "wifi")]
    WiFi,
}

/// One IPv4 address of this machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalAddr {
    pub interface: String,
    pub ip: Ipv4Addr,
    pub prefix: u8,
    pub kind: LinkKind,
}

impl LocalAddr {
    fn contains(&self, ip: Ipv4Addr) -> bool {
        let mask = u32::MAX
            .checked_shl(32 - u32::from(self.prefix))
            .unwrap_or(0);
        u32::from(self.ip) & mask == u32::from(ip) & mask
    }
}

/// Classifies an interface from its name and address.
pub fn classify(interface: &str, ip: Ipv4Addr) -> LinkKind {
    if ip.is_link_local() {
        return LinkKind::Direct;
    }
    let name = interface.to_ascii_lowercase();
    let has = |words: &[&str]| words.iter().any(|w| name.contains(w));
    // Linux: wl*, thunderbolt*, en*/eth*. Windows: friendly names such as
    // "Wi-Fi", "Ethernet 2". macOS: bridge* is the Thunderbolt Bridge.
    if has(&["wi-fi", "wifi", "wlan", "wireless"]) || name.starts_with("wl") {
        LinkKind::WiFi
    } else if has(&["thunderbolt"]) || name.starts_with("bridge") {
        LinkKind::Thunderbolt
    } else if has(&["ethernet"]) || name.starts_with("eth") || name.starts_with("enp") {
        LinkKind::Wired
    } else {
        LinkKind::Other
    }
}

/// This machine's IPv4 addresses, loopback and virtual-looking ones left
/// out, best links first.
pub fn local_ipv4() -> Vec<LocalAddr> {
    let mut out: Vec<LocalAddr> = if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter(|i| !i.is_loopback())
        .filter_map(|i| match i.addr {
            if_addrs::IfAddr::V4(v4) => Some(LocalAddr {
                kind: classify(&i.name, v4.ip),
                interface: i.name,
                ip: v4.ip,
                prefix: v4.prefixlen,
            }),
            if_addrs::IfAddr::V6(_) => None,
        })
        .filter(|a| !is_virtual(&a.interface))
        .collect();
    out.sort_by_key(|a| (a.kind, a.interface.clone()));
    out
}

/// Container and VM bridges: their addresses are not reachable from other
/// machines and would only confuse the list.
fn is_virtual(interface: &str) -> bool {
    let name = interface.to_ascii_lowercase();
    [
        "docker",
        "veth",
        "virbr",
        "vmnet",
        "vboxnet",
        "utun",
        "tailscale",
        "zt",
    ]
    .iter()
    .any(|p| name.starts_with(p))
        || name.contains("vethernet")
}

/// Orders a master's `candidates` so the best path from this machine comes
/// first: a cable straight to it, then a subnet shared with one of this
/// machine's wired links, then any shared subnet, then the rest. Ties keep
/// the order they came in.
pub fn rank(candidates: &[Ipv4Addr], local: &[LocalAddr]) -> Vec<Ipv4Addr> {
    let score = |ip: Ipv4Addr| {
        let on = local
            .iter()
            .filter(|l| l.contains(ip))
            .map(|l| l.kind)
            .min();
        match on {
            Some(LinkKind::Direct) => 0,
            Some(LinkKind::Thunderbolt | LinkKind::Wired) => 1,
            Some(LinkKind::Other) => 2,
            Some(LinkKind::WiFi) => 3,
            None => 4,
        }
    };
    let mut ranked: Vec<Ipv4Addr> = Vec::new();
    for ip in candidates {
        if !ranked.contains(ip) {
            ranked.push(*ip);
        }
    }
    ranked.sort_by_key(|ip| score(*ip));
    ranked
}

/// The first of `addrs` that accepts a TCP connection within `timeout`
/// each. Only checks that something listens; the agent then proves itself
/// to the master as always.
pub async fn first_reachable(addrs: &[SocketAddr], timeout: Duration) -> Option<SocketAddr> {
    for addr in addrs {
        if let Ok(Ok(_)) = tokio::time::timeout(timeout, tokio::net::TcpStream::connect(addr)).await
        {
            return Some(*addr);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn local(interface: &str, ip: [u8; 4], prefix: u8) -> LocalAddr {
        let ip = Ipv4Addr::from(ip);
        LocalAddr {
            interface: interface.into(),
            kind: classify(interface, ip),
            ip,
            prefix,
        }
    }

    #[test]
    fn interfaces_are_told_apart_by_name_and_address() {
        let ip = Ipv4Addr::new(192, 168, 0, 5);
        assert_eq!(classify("wlp2s0", ip), LinkKind::WiFi);
        assert_eq!(classify("Wi-Fi", ip), LinkKind::WiFi);
        assert_eq!(classify("enp3s0", ip), LinkKind::Wired);
        assert_eq!(classify("Ethernet 2", ip), LinkKind::Wired);
        assert_eq!(classify("eth0", ip), LinkKind::Wired);
        assert_eq!(classify("bridge0", ip), LinkKind::Thunderbolt);
        assert_eq!(classify("thunderbolt0", ip), LinkKind::Thunderbolt);
        assert_eq!(
            classify("en0", ip),
            LinkKind::Other,
            "a Mac's en0 can be either"
        );
        assert_eq!(
            classify("en0", Ipv4Addr::new(169, 254, 3, 4)),
            LinkKind::Direct
        );
        assert!(is_virtual("docker0") && is_virtual("vEthernet (WSL)"));
        assert!(!is_virtual("en0"));
        assert_eq!(serde_json::to_string(&LinkKind::WiFi).unwrap(), r#""wifi""#);
        assert_eq!(
            serde_json::to_string(&LinkKind::Direct).unwrap(),
            r#""direct""#
        );
    }

    #[test]
    fn a_cable_beats_wired_lan_beats_wifi_beats_strangers() {
        let me = [
            local("wlan0", [192, 168, 0, 10], 24),
            local("eth0", [10, 0, 0, 10], 24),
            local("bridge0", [169, 254, 7, 1], 16),
        ];
        let master = [
            Ipv4Addr::new(172, 16, 0, 1),   // no shared subnet
            Ipv4Addr::new(192, 168, 0, 20), // over Wi-Fi
            Ipv4Addr::new(10, 0, 0, 20),    // over the LAN cable
            Ipv4Addr::new(169, 254, 9, 9),  // over the Thunderbolt cable
            Ipv4Addr::new(10, 0, 0, 20),    // duplicate
        ];
        assert_eq!(
            rank(&master, &me),
            vec![
                Ipv4Addr::new(169, 254, 9, 9),
                Ipv4Addr::new(10, 0, 0, 20),
                Ipv4Addr::new(192, 168, 0, 20),
                Ipv4Addr::new(172, 16, 0, 1),
            ]
        );
        // With nothing known about this machine the order stays as given.
        assert_eq!(rank(&master[..2], &[]), master[..2].to_vec());
    }

    #[test]
    fn subnets_follow_the_prefix() {
        let wide = local("eth0", [10, 1, 2, 3], 8);
        assert!(wide.contains(Ipv4Addr::new(10, 200, 0, 1)));
        let narrow = local("eth0", [10, 1, 2, 3], 30);
        assert!(!narrow.contains(Ipv4Addr::new(10, 1, 2, 8)));
        let any = local("eth0", [10, 1, 2, 3], 0);
        assert!(any.contains(Ipv4Addr::new(8, 8, 8, 8)));
    }

    #[tokio::test]
    async fn the_first_address_that_answers_is_used() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let up = listener.local_addr().unwrap();
        let free = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let down = free.local_addr().unwrap();
        drop(free);
        let timeout = Duration::from_millis(500);
        assert_eq!(first_reachable(&[down, up], timeout).await, Some(up));
        assert_eq!(first_reachable(&[down], timeout).await, None);
    }
}
