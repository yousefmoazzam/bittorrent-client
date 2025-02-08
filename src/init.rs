use crate::{
    metainfo::Metainfo,
    torrent::Torrent,
    tracker::{Request, Response},
    PEER_ID,
};

/// Perform required setup prior to attempting file download
pub async fn init(metainfo: Metainfo) -> Torrent {
    let bencoded_info = crate::encode::encode(metainfo.info.serialise());
    let info_hash = sha1_smol::Sha1::from(bencoded_info).digest().bytes();
    let request = Request::new(
        &metainfo.announce,
        PEER_ID,
        6881,
        info_hash.to_vec(),
        metainfo.info.length,
    )
    .unwrap();
    let data = &request.send().await.unwrap();
    let resp = Response::deserialise(data);
    match resp {
        Response::Failure(_) => todo!(),
        Response::Success { peers, .. } => Torrent::new(metainfo, peers),
    }
}

#[cfg(test)]
mod tests {
    use super::init;
    use crate::{
        metainfo::{Info, Metainfo},
        torrent::Torrent,
        tracker::Peer,
    };
    use mockito::Matcher::UrlEncoded;
    use std::net::Ipv4Addr;

    #[tokio::test]
    async fn generate_correct_torrent_struct_from_tracker_response_and_metainfo() {
        let file_len = 128;
        let tracker_port = 1234;
        let client_port = 6881;
        let server_opts = mockito::ServerOpts {
            port: tracker_port,
            ..Default::default()
        };
        let mut tracker = mockito::Server::new_with_opts_async(server_opts).await;

        let metainfo = Metainfo {
            announce: tracker.url(),
            info: Info {
                name: "file".to_string(),
                length: file_len,
                piece_length: 64,
                pieces: format!(
                    "{}{}",
                    "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d",
                    "3c8ec4874488f6090a157b014ce3397ca8e06d4f"
                ),
            },
        };
        let bencoded_info = crate::encode::encode(metainfo.info.serialise());
        let info_hash = sha1_smol::Sha1::from(bencoded_info).digest().bytes();
        let info_hash_str = info_hash
            .iter()
            .map(|byte| format!("%{:02x}", byte))
            .collect::<Vec<String>>()
            .join("");

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
        let response_data = format!(
            "d{}:{}i{}e{}:{}{}:{}e",
            "interval".len(),
            "interval",
            interval_value,
            "peers".len(),
            "peers",
            peers_data.len(),
            peers_data
        );
        let mock = tracker
            .mock("GET", "/")
            .match_query(mockito::Matcher::AllOf(vec![
                mockito::Matcher::Regex(format!("info_hash={}", info_hash_str)),
                UrlEncoded("peer_id".to_string(), crate::PEER_ID.to_string()),
                UrlEncoded("port".to_string(), client_port.to_string()),
                UrlEncoded("uploaded".to_string(), 0.to_string()),
                UrlEncoded("downloaded".to_string(), 0.to_string()),
                UrlEncoded("compact".to_string(), 1.to_string()),
                UrlEncoded("left".to_string(), file_len.to_string()),
            ]))
            .with_body(response_data)
            .create();

        let torrent = init(metainfo.clone()).await;
        let expected_torrent = Torrent {
            metainfo,
            info_hash: info_hash.to_vec(),
            peers: vec![
                Peer {
                    ip: Ipv4Addr::new(192, 0, 2, 123),
                    port: 6881,
                },
                Peer {
                    ip: Ipv4Addr::new(192, 0, 2, 124),
                    port: 6881,
                },
            ],
        };
        mock.assert_async().await;
        assert_eq!(torrent.peers, expected_torrent.peers);
    }
}
