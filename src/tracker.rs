use std::net::Ipv4Addr;

use crate::BencodeType;

const INTERVAL_KEY: &str = "interval";
const PEERS_KEY: &str = "peers";
const PEER_NO_OF_HEX_DIGITS: usize = 12;

/// Tracker associated with file
pub struct Tracker {
    /// Interval (in seconds) at which to reconnect to tracker to refresh peer list
    pub interval: usize,
    /// Peers of file reported by tracker
    pub peers: Vec<Peer>,
}

impl Tracker {
    pub fn new(data: BencodeType) -> Tracker {
        match data {
            BencodeType::Dict(mut map) => {
                let interval: usize = match map.remove(INTERVAL_KEY).unwrap() {
                    BencodeType::Integer(int) => int.try_into().unwrap(),
                    _ => todo!(),
                };
                let peer_data = match map.remove(PEERS_KEY).unwrap() {
                    BencodeType::ByteString(string) => string,
                    _ => todo!(),
                };
                let peers = Self::parse_peers(&peer_data);
                Tracker { interval, peers }
            }
            _ => todo!(),
        }
    }

    fn parse_peers(data: &str) -> Vec<Peer> {
        let mut peers = Vec::new();
        let mut idx = 0;
        while idx < data.len() {
            let peer_substring = &data[idx..idx + PEER_NO_OF_HEX_DIGITS];
            peers.push(Peer::new(peer_substring));
            idx += PEER_NO_OF_HEX_DIGITS;
        }
        peers
    }
}

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
    use std::collections::HashMap;

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

    #[test]
    fn parse_tracker_from_bencoded_data() {
        let interval = 900;
        let peer_one_ip = [0xC0, 0x00, 0x02, 0x7B]; // 192.0.2.123
        let peer_two_ip = [0xC0, 0x00, 0x02, 0x7C]; // 192.0.2.124
        let port_bytes_network_order = [0x1A, 0xE1]; // 6881
        let mut peers_data = Vec::new();
        for ip in [peer_one_ip, peer_two_ip] {
            peers_data.push(format!(
                "{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
                ip[0],
                ip[1],
                ip[2],
                ip[3],
                port_bytes_network_order[0],
                port_bytes_network_order[1]
            ));
        }
        let peers_data = peers_data.join("");
        let mut map = HashMap::new();
        map.insert("interval".to_string(), BencodeType::Integer(interval));
        map.insert("peers".to_string(), BencodeType::ByteString(peers_data));
        let response_dict = BencodeType::Dict(map);
        let tracker = Tracker::new(response_dict);
        assert_eq!(interval, tracker.interval.try_into().unwrap());
        assert_eq!(2, tracker.peers.len());
    }
}
