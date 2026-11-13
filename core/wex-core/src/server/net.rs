//! Addresses friends use to reach the server.

use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream, UdpSocket};
use std::time::Duration;

/// The IPv4 address of the interface that routes outward, i.e. the one a
/// friend in the same network reaches. A UDP `connect` only selects a route;
/// no packet is sent.
pub fn lan_ipv4() -> Option<Ipv4Addr> {
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).ok()?;
    socket.connect((Ipv4Addr::new(192, 0, 2, 1), 9)).ok()?;
    match socket.local_addr().ok()?.ip() {
        IpAddr::V4(ip) if is_lan_candidate(ip) => Some(ip),
        _ => None,
    }
}

pub fn is_lan_candidate(ip: Ipv4Addr) -> bool {
    !ip.is_loopback() && !ip.is_unspecified() && !ip.is_link_local() && !ip.is_multicast()
}

/// `host` with `:port` unless the port is the Minecraft default.
pub fn join_address(host: &str, port: u16) -> String {
    let has_port = if host.starts_with('[') {
        host.contains("]:")
    } else {
        host.matches(':').count() == 1
            && host
                .rsplit_once(':')
                .is_some_and(|(_, p)| p.parse::<u16>().is_ok())
    };
    if has_port {
        return host.to_string();
    }
    let h = if !host.starts_with('[') && host.contains(':') {
        format!("[{host}]")
    } else {
        host.to_string()
    };
    if port == 25565 {
        h
    } else {
        format!("{h}:{port}")
    }
}

/// Something already answers on the port on this PC. A connect probe, not a
/// bind: binding a listening socket here could raise a firewall prompt.
pub fn port_in_use(port: u16) -> bool {
    TcpStream::connect_timeout(
        &SocketAddr::from((Ipv4Addr::LOCALHOST, port)),
        Duration::from_millis(300),
    )
    .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn address_formatting() {
        assert_eq!(join_address("192.168.1.5", 25565), "192.168.1.5");
        assert_eq!(join_address("192.168.1.5", 25570), "192.168.1.5:25570");
        assert_eq!(
            join_address("play.example.com:40000", 25570),
            "play.example.com:40000"
        );
        assert_eq!(join_address("::1", 25570), "[::1]:25570");
    }

    #[test]
    fn lan_candidates() {
        assert!(is_lan_candidate(Ipv4Addr::new(192, 168, 0, 10)));
        assert!(!is_lan_candidate(Ipv4Addr::LOCALHOST));
        assert!(!is_lan_candidate(Ipv4Addr::new(169, 254, 1, 1)));
        if let Some(ip) = lan_ipv4() {
            assert!(is_lan_candidate(ip));
        }
    }

    #[test]
    fn port_probe() {
        let listener = std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        assert!(port_in_use(port));
        drop(listener);
        assert!(!port_in_use(port));
    }
}
