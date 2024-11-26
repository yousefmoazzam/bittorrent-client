use std::net::Ipv4Addr;

/// Peer of file
#[derive(Debug, PartialEq)]
struct Peer {
    /// IP address of peer
    ip: Ipv4Addr,
    /// Port of peer
    port: u16,
}

impl Peer {
    fn new(data: &str) -> Peer {
        let mut bytes = Vec::new();
        let mut idx = 0;

        while idx < data.len() {
            let substring = &data[idx..idx + 2];
            bytes.push(substring);
            idx += 2;
        }

        let ip = bytes[..4]
            .iter()
            .map(|string| u8::from_str_radix(string, 16).unwrap())
            .collect::<Vec<_>>();
        let port = u16::from_str_radix(&bytes[4..].join(""), 16).unwrap();

        Peer {
            ip: Ipv4Addr::new(ip[0], ip[1], ip[2], ip[3]),
            port,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_peer() {
        let ip = [0xC0, 0x00, 0x02, 0x7B]; // 192.0.2.123
        let port_bytes_network_order = [0x1A, 0xE1]; // 6881
        let peers_data = format!(
            "{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            ip[0], ip[1], ip[2], ip[3], port_bytes_network_order[0], port_bytes_network_order[1]
        );
        let expected_peer = Peer {
            ip: Ipv4Addr::new(ip[0], ip[1], ip[2], ip[3]),
            port: u16::from_be_bytes(port_bytes_network_order),
        };
        let peer = Peer::new(&peers_data);
        assert_eq!(peer, expected_peer);
    }
}
