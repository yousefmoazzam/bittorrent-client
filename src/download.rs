use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::handshake::Handshake;
use crate::torrent::Torrent;
use crate::{HANDSHAKE_BYTES_LEN, PEER_ID, PSTR};

/// Download file
pub async fn download(torrent: Torrent) -> Vec<u8> {
    let buf = vec![0; torrent.metainfo.info.length];

    for peer in torrent.peers {
        let addr = format!("{}:{}", peer.ip, peer.port);
        let mut socket = tokio::net::TcpStream::connect(addr).await.unwrap();
        let initial_handshake =
            Handshake::new(PSTR.to_string(), torrent.info_hash.clone(), PEER_ID.into());
        socket
            .write_all(&initial_handshake.serialise())
            .await
            .unwrap();
        let mut buf = [0; HANDSHAKE_BYTES_LEN];
        socket.read_exact(&mut buf).await.unwrap();
    }

    buf
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use tokio::io::AsyncReadExt;
    use tokio::net::TcpListener;

    use crate::message::Message;
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

    #[tokio::test]
    async fn downloads_file_from_single_peer() {
        // Setup file data
        const NO_OF_PIECES: u8 = 2;
        const PIECE_LEN: u8 = 64;
        const FILE_LEN: i64 = PIECE_LEN as i64 * NO_OF_PIECES as i64;
        let piece_template = (0..PIECE_LEN).collect::<Vec<u8>>();
        let mut original_data = [0; NO_OF_PIECES as usize * PIECE_LEN as usize];
        let mut count = 0;
        let mut idx = 0;
        let mut pieces = Vec::new();
        while idx < original_data.len() {
            let piece = piece_template[..]
                .iter()
                .map(|val| val + count * 10)
                .collect::<Vec<u8>>();
            original_data[idx..idx + PIECE_LEN as usize].copy_from_slice(&piece);
            pieces.push(piece);
            count += 1;
            idx += PIECE_LEN as usize;
        }

        // Calculate SHA1 hashes of the pieces
        let mut piece_hashes = Vec::new();
        for idx in 0..NO_OF_PIECES {
            let piece = &original_data
                [idx as usize * PIECE_LEN as usize..(idx as usize + 1) * PIECE_LEN as usize];
            let hash = sha1_smol::Sha1::from(piece).digest().to_string();
            piece_hashes.push(hash);
        }

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
        let mut info_map = HashMap::new();
        info_map.insert(
            "name".to_string(),
            BencodeType::ByteString(name.to_string()),
        );
        info_map.insert("length".to_string(), BencodeType::Integer(FILE_LEN));
        info_map.insert(
            "piece length".to_string(),
            BencodeType::Integer(PIECE_LEN as i64),
        );
        info_map.insert(
            "pieces".to_string(),
            BencodeType::ByteString(format!("{}{}", piece_hashes[0], piece_hashes[1])),
        );
        metainfo_map.insert("info".to_string(), BencodeType::Dict(info_map));
        let data = BencodeType::Dict(metainfo_map);
        let metainfo = Metainfo::new(data).unwrap();
        let torrent = Torrent::new(metainfo, peers);

        let initial_handshake =
            Handshake::new(PSTR.to_string(), torrent.info_hash.clone(), PEER_ID.into());
        let initial_handshake_data = initial_handshake.serialise();
        let mut buf = vec![0; initial_handshake_data.len()];
        let len: u32 = 2;
        let id = 0x05;
        let payload = vec![0b11000000];
        let mut bitfield_message = u32::to_be_bytes(len).to_vec();
        bitfield_message.push(id);
        bitfield_message.append(&mut payload.clone());

        let piece_zero_requests = [
            Message::Request {
                index: 0,
                begin: 0,
                length: PIECE_LEN as u64 / 2,
            },
            Message::Request {
                index: 0,
                begin: PIECE_LEN as u64 / 2,
                length: PIECE_LEN as u64 / 2,
            },
        ];
        let piece_zero_responses = [
            Message::Piece {
                index: 0,
                begin: 0,
                block: original_data[..PIECE_LEN as usize / 2].to_vec(),
            },
            Message::Piece {
                index: 0,
                begin: PIECE_LEN as u64 / 2,
                block: original_data[PIECE_LEN as usize / 2..PIECE_LEN as usize].to_vec(),
            },
        ];
        let piece_one_requests = [
            Message::Request {
                index: 1,
                begin: 0,
                length: PIECE_LEN as u64 / 2,
            },
            Message::Request {
                index: 1,
                begin: PIECE_LEN as u64 / 2,
                length: PIECE_LEN as u64 / 2,
            },
        ];
        let piece_one_responses = [
            Message::Piece {
                index: 1,
                begin: 0,
                block: original_data
                    [PIECE_LEN as usize..PIECE_LEN as usize + PIECE_LEN as usize / 2]
                    .to_vec(),
            },
            Message::Piece {
                index: 1,
                begin: PIECE_LEN as u64 / 2,
                block: original_data
                    [PIECE_LEN as usize + PIECE_LEN as usize / 2..PIECE_LEN as usize * 2]
                    .to_vec(),
            },
        ];

        let mut throwaway_buf = [0; 1024];
        let mock_peer_socket_handle = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let _ = socket.read(&mut throwaway_buf).await.unwrap();
            socket.write_all(&bitfield_message).await.unwrap();
            let _ = socket.read(&mut throwaway_buf).await.unwrap();
            let _ = socket.read(&mut throwaway_buf).await.unwrap();
            socket
                .write_all(&pieces[0][..PIECE_LEN as usize / 2])
                .await
                .unwrap();
            socket
                .write_all(&pieces[0][PIECE_LEN as usize / 2..PIECE_LEN as usize])
                .await
                .unwrap();
            let _ = socket.read(&mut throwaway_buf).await.unwrap();
            let _ = socket.read(&mut throwaway_buf).await.unwrap();
            socket
                .write_all(&pieces[1][..PIECE_LEN as usize / 2])
                .await
                .unwrap();
            socket
                .write_all(&pieces[1][PIECE_LEN as usize / 2..PIECE_LEN as usize])
                .await
                .unwrap();
        });

        let res = download(torrent).await;
        assert_eq!(res, original_data);
    }
}
