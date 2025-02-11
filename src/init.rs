use crate::{
    metainfo::Metainfo,
    torrent::Torrent,
    tracker::{Request, Response},
    PEER_ID,
};

/// Perform required setup prior to attempting file download
pub async fn init(metainfo: Metainfo) -> Torrent {
    let bencoded_info = crate::serialise::serialise(metainfo.info.serialise());
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

        let piece_one_hash =
            b"\xaa\xf4\xc6\x1d\xdc\xc5\xe8\xa2\xda\xbe\xde\x0f\x3b\x48\x2c\xd9\xae\xa9\x43\x4d";
        let piece_two_hash =
            b"\x3c\x8e\xc4\x87\x44\x88\xf6\x09\x0a\x15\x7b\x01\x4c\xe3\x39\x7c\xa8\xe0\x6d\x4f";
        let mut piece_hashes = piece_one_hash.to_vec();
        piece_hashes.append(&mut piece_two_hash.to_vec());
        let metainfo = Metainfo {
            announce: tracker.url(),
            info: Info {
                name: "file".to_string(),
                length: file_len,
                piece_length: 64,
                pieces: piece_hashes,
            },
        };
        let bencoded_info = crate::serialise::serialise(metainfo.info.serialise());
        let info_hash = sha1_smol::Sha1::from(bencoded_info).digest().bytes();
        let info_hash_str = info_hash
            .iter()
            .map(|byte| format!("%{:02x}", byte))
            .collect::<Vec<String>>()
            .join("");

        let peer_one_ip = [0xC0, 0x00, 0x02, 0x7B]; // 192.0.2.123
        let peer_two_ip = [0xC0, 0x00, 0x02, 0x7C]; // 192.0.2.124
        let port_bytes_network_order = [0x1A, 0xE1]; // 6881
        let mut peers_data = Vec::new();
        peers_data.append(&mut peer_one_ip.to_vec());
        peers_data.append(&mut port_bytes_network_order.to_vec());
        peers_data.append(&mut peer_two_ip.to_vec());
        peers_data.append(&mut port_bytes_network_order.to_vec());
        let response_data =
            b"d8:intervali900e5:peers12:\xc0\x00\x02\x7b\x1a\xe1\xc0\x00\x02\x7c\x1a\xe1e";
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
