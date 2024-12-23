use std::net::Ipv4Addr;

use reqwest::Url;

use crate::BencodeType;

const INTERVAL_KEY: &str = "interval";
const PEERS_KEY: &str = "peers";
const PEER_NO_OF_HEX_DIGITS: usize = 12;

/// GET request to tracker
pub struct Request {
    /// URL to make GET request to tracker
    pub url: Url,
}

impl Request {
    /// Create request
    pub fn new(
        tracker_url: &str,
        peer_id: &str,
        port: u32,
        info_hash: Vec<u8>,
        file_length: usize,
    ) -> Request {
        let info_hash_str = info_hash
            .iter()
            .map(|byte| format!("%{:02x}", byte))
            .collect::<Vec<String>>()
            .join("");
        let mut string_url = tracker_url.to_string();
        string_url.push_str("?info_hash=");
        string_url.push_str(&info_hash_str);
        let mut url = Url::parse(&string_url).unwrap();
        url.query_pairs_mut()
            .append_pair("peer_id", peer_id)
            .append_pair("port", &port.to_string())
            .append_pair("uploaded", &0.to_string())
            .append_pair("downloaded", &0.to_string())
            .append_pair("compact", &1.to_string())
            .append_pair("left", &file_length.to_string());
        Request { url }
    }

    /// Send request and return response body
    pub async fn send(self) -> reqwest::Result<String> {
        let response = reqwest::get(self.url).await?;
        response.text().await
    }
}

/// Response from tracker
pub enum Response {
    /// Failed query
    Failure(String),
    /// Successful query
    Success {
        /// Interval (in seconds) at which to reconnect to tracker to refresh peer list
        interval: usize,
        /// Peers of file reported by tracker
        peers: Vec<Peer>,
    },
}

impl Response {
    /// Deserialise response message body
    pub fn deserialise(data: &str) -> Response {
        match crate::decode::decode(data) {
            BencodeType::Dict(map) => {
                if let Some(val) = map.get("failure") {
                    match val {
                        BencodeType::ByteString(msg) => return Response::Failure(msg.to_string()),
                        _ => todo!(),
                    };
                };

                let interval: usize = match map.get(INTERVAL_KEY).unwrap() {
                    BencodeType::Integer(int) => *int as usize,
                    _ => todo!(),
                };
                let peer_data = match map.get(PEERS_KEY).unwrap() {
                    BencodeType::ByteString(string) => string,
                    _ => todo!(),
                };
                let peers = Self::parse_peers(&peer_data[..]);
                Response::Success { interval, peers }
            }
            _ => todo!(),
        }
    }

    /// Parse peers encoded in "compact" form
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
pub struct Peer {
    /// IP address of peer
    pub ip: Ipv4Addr,
    /// Port of peer
    pub port: u16,
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
    use mockito::Matcher::UrlEncoded;

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
    fn create_success_variant_from_successful_response() {
        let interval_value = 900;
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
        let bencoded_data = format!(
            "d{}:{}i{}e{}:{}{}:{}e",
            INTERVAL_KEY.len(),
            INTERVAL_KEY,
            interval_value,
            PEERS_KEY.len(),
            PEERS_KEY,
            peers_data.len(),
            peers_data
        );
        let response = Response::deserialise(&bencoded_data);
        match response {
            Response::Success { interval, peers } => {
                assert_eq!(interval_value, TryInto::<i64>::try_into(interval).unwrap());
                assert_eq!(2, peers.len());
            }
            _ => panic!("Expected successful response"),
        }
    }

    #[test]
    fn creating_tracker_request_produces_expected_url_for_get_request() {
        let tracker_url = "http://a.b.org:1234/announce";
        let info_hash = (0x00..0x14).collect::<Vec<u8>>();
        let port = 6881;
        let file_length = 128;
        let request = Request::new(
            tracker_url,
            crate::PEER_ID,
            port,
            info_hash.clone(),
            file_length,
        );

        let info_hash_str = info_hash
            .iter()
            .map(|byte| format!("%{:02x}", byte))
            .collect::<Vec<String>>()
            .join("");
        let mut string_url = tracker_url.to_string();
        string_url.push_str("?info_hash=");
        string_url.push_str(&info_hash_str);
        let mut expected_url = Url::parse(&string_url).unwrap();
        expected_url
            .query_pairs_mut()
            .append_pair("peer_id", crate::PEER_ID)
            .append_pair("port", &port.to_string())
            .append_pair("uploaded", &0.to_string())
            .append_pair("downloaded", &0.to_string())
            .append_pair("compact", &1.to_string())
            .append_pair("left", &file_length.to_string());
        assert_eq!(request.url, expected_url);
    }

    #[tokio::test]
    async fn sent_tracker_get_request_is_received_by_tracker() {
        let info_hash = (0x00..0x14).collect::<Vec<u8>>();
        let port = 6881;
        let file_length = 128;
        let info_hash_str = info_hash
            .iter()
            .map(|byte| format!("%{:02x}", byte))
            .collect::<Vec<String>>()
            .join("");
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/")
            .match_query(mockito::Matcher::AllOf(vec![
                mockito::Matcher::Regex(format!("info_hash={}", info_hash_str)),
                UrlEncoded("peer_id".to_string(), crate::PEER_ID.to_string()),
                UrlEncoded("port".to_string(), port.to_string()),
                UrlEncoded("uploaded".to_string(), 0.to_string()),
                UrlEncoded("downloaded".to_string(), 0.to_string()),
                UrlEncoded("compact".to_string(), 1.to_string()),
                UrlEncoded("left".to_string(), file_length.to_string()),
            ]))
            .create();

        Request::new(
            &server.url(),
            crate::PEER_ID,
            port,
            info_hash.clone(),
            file_length,
        )
        .send()
        .await
        .unwrap();
        mock.assert();
    }

    #[tokio::test]
    async fn sent_request_returns_response_body() {
        let info_hash = (0x00..0x14).collect::<Vec<u8>>();
        let port = 6881;
        let file_length = 128;
        let info_hash_str = info_hash
            .iter()
            .map(|byte| format!("%{:02x}", byte))
            .collect::<Vec<String>>()
            .join("");

        let key = "failure";
        let value = "Some reason for query failure";
        let bencoded_data = format!("d{}:{}{}:{}e", key.len(), key, value.len(), value);

        let mut server = mockito::Server::new_async().await;
        server
            .mock("GET", "/")
            .match_query(mockito::Matcher::AllOf(vec![
                mockito::Matcher::Regex(format!("info_hash={}", info_hash_str)),
                UrlEncoded("peer_id".to_string(), crate::PEER_ID.to_string()),
                UrlEncoded("port".to_string(), port.to_string()),
                UrlEncoded("uploaded".to_string(), 0.to_string()),
                UrlEncoded("downloaded".to_string(), 0.to_string()),
                UrlEncoded("compact".to_string(), 1.to_string()),
                UrlEncoded("left".to_string(), file_length.to_string()),
            ]))
            .with_body(bencoded_data.clone())
            .create();
        let response = Request::new(&server.url(), crate::PEER_ID, port, info_hash, file_length)
            .send()
            .await
            .unwrap();
        assert_eq!(response, bencoded_data);
    }

    #[test]
    fn create_error_variant_from_failure_response() {
        let key = "failure";
        let value = "Some reason for query failure";
        let bencoded_data = format!("d{}:{}{}:{}e", key.len(), key, value.len(), value);
        let response = Response::deserialise(&bencoded_data);
        match response {
            Response::Failure(msg) => assert_eq!(msg, value),
            _ => panic!("Expected failure response"),
        }
    }
}
