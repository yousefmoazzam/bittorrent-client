use tokio::io::AsyncWriteExt;

use crate::handshake::Handshake;
use crate::torrent::Torrent;
use crate::{PEER_ID, PSTR};

/// Download file
pub async fn download(torrent: Torrent) {
    for peer in torrent.peers {
        let addr = format!("{}:{}", peer.ip, peer.port);
        let mut socket = tokio::net::TcpStream::connect(addr).await.unwrap();
        let initial_handshake =
            Handshake::new(PSTR.to_string(), torrent.info_hash.clone(), PEER_ID.into());
        socket
            .write_all(&initial_handshake.serialise())
            .await
            .unwrap();
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use tokio::io::AsyncReadExt;
    use tokio::net::TcpListener;

    use crate::metainfo::Metainfo;
    use crate::tracker::Peer;
    use crate::BencodeType;

    use super::*;

    #[tokio::test]
    async fn sends_handshake_to_peer() {
        let ip = "127.0.0.1";
        let port = 12345;
        let addr = format!("{}:{}", ip, port);
        let listener = TcpListener::bind(addr).await.unwrap();
        let peers = vec![Peer {
            ip: std::net::Ipv4Addr::new(127, 0, 0, 1),
            port,
        }];

        let announce = "hello";
        let mut metainfo_map = HashMap::new();
        metainfo_map.insert(
            "announce".to_string(),
            BencodeType::ByteString(announce.to_string()),
        );

        let name = "hello";
        let length = 128;
        let piece_length = 64;
        let hello_sha1 = "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d";
        let goodbye_sha1 = "3c8ec4874488f6090a157b014ce3397ca8e06d4f";
        let mut info_map = HashMap::new();
        info_map.insert(
            "name".to_string(),
            BencodeType::ByteString(name.to_string()),
        );
        info_map.insert("length".to_string(), BencodeType::Integer(length));
        info_map.insert(
            "piece length".to_string(),
            BencodeType::Integer(piece_length),
        );
        info_map.insert(
            "pieces".to_string(),
            BencodeType::ByteString(format!("{}{}", hello_sha1, goodbye_sha1)),
        );
        metainfo_map.insert("info".to_string(), BencodeType::Dict(info_map));
        let data = BencodeType::Dict(metainfo_map);
        let metainfo = Metainfo::new(data).unwrap();
        let torrent = Torrent::new(metainfo, peers);

        // Launch tokio task to accept a TCP connection when the client requests a connection
        // during downloading
        let initial_handshake =
            Handshake::new(PSTR.to_string(), torrent.info_hash.clone(), PEER_ID.into());
        let initial_handshake_data = initial_handshake.serialise();
        let mut buf = vec![0; initial_handshake_data.len()];
        let mock_peer_socket_handle = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            socket.read_exact(&mut buf).await.unwrap();
            buf
        });

        download(torrent).await;
        let filled_buf = mock_peer_socket_handle.await.unwrap();

        assert_eq!(filled_buf, initial_handshake_data);
    }
}
