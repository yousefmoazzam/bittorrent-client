use crate::client::Client;
use crate::torrent::Torrent;
use crate::work::{SharedQueue, Work};
use crate::worker::Worker;
use tracing::{info, warn};

/// Download file
pub async fn download(torrent: Torrent) -> Vec<u8> {
    let work = torrent
        .metainfo
        .info
        .pieces()
        .enumerate()
        .map(|(idx, hash)| Work {
            index: u64::try_from(idx).unwrap(),
            length: u64::try_from(torrent.metainfo.info.piece_length).unwrap(),
            hash: hash.to_vec(),
        })
        .collect::<Vec<_>>();
    let no_of_pieces = work.len();
    let queue = SharedQueue::new(work);
    let (tx, rx) = tokio::sync::mpsc::channel(no_of_pieces);

    for peer in torrent.peers {
        let info_hash = torrent.info_hash.clone();
        let queue = queue.clone();
        let tx = tx.clone();
        let addr = format!("{}:{}", peer.ip, peer.port);
        match tokio::net::TcpStream::connect(addr).await {
            Err(e) => {
                warn!(
                    "Unable to establish TCP connection with {}:{}, got error: {}",
                    peer.ip, peer.port, e
                );
            }
            Ok(socket) => {
                info!("Established TCP connection with {}:{}", peer.ip, peer.port);
                tokio::spawn(async move {
                    let client = Client::new(socket, info_hash).await.unwrap();
                    let mut worker = Worker::new(client, tx, queue);
                    worker.download().await.unwrap();
                });
            }
        };
    }

    drop(tx);
    let mut buf = vec![0; torrent.metainfo.info.length];
    crate::piece::receiver(&mut buf, torrent.metainfo.info.piece_length, rx).await;
    buf
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::io::{Read, Write};

    use tokio::io::AsyncReadExt;
    use tokio::net::TcpListener;

    use crate::handshake::Handshake;
    use crate::message::Message;
    use crate::metainfo::Metainfo;
    use crate::parse::BencodeType2;
    use crate::tracker::Peer;
    use crate::{HANDSHAKE_BYTES_LEN, PEER_ID, PSTR};

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

        let announce = b"hello";
        let mut metainfo_map = HashMap::new();
        metainfo_map.insert(
            b"announce".to_vec(),
            BencodeType2::ByteString(announce.to_vec()),
        );

        let name = b"hello";
        let length = 128;
        let piece_length = 64;
        let hello_sha1 =
            b"\xaa\xf4\xc6\x1d\xdc\xc5\xe8\xa2\xda\xbe\xde\x0f\x3b\x48\x2c\xd9\xae\xa9\x43\x4d";
        let goodbye_sha1 =
            b"\x3c\x8e\xc4\x87\x44\x88\xf6\x09\x0a\x15\x7b\x01\x4c\xe3\x39\x7c\xa8\xe0\x6d\x4f";
        let mut piece_hashes = hello_sha1.to_vec();
        piece_hashes.append(&mut goodbye_sha1.to_vec());
        let mut info_map = HashMap::new();
        info_map.insert(b"name".to_vec(), BencodeType2::ByteString(name.to_vec()));
        info_map.insert(b"length".to_vec(), BencodeType2::Integer(length));
        info_map.insert(
            b"piece length".to_vec(),
            BencodeType2::Integer(piece_length),
        );
        info_map.insert(
            b"pieces".to_vec(),
            BencodeType2::ByteString(piece_hashes.clone()),
        );
        metainfo_map.insert(b"info".to_vec(), BencodeType2::Dict(info_map));
        let data = BencodeType2::Dict(metainfo_map);
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
        const NO_OF_PIECES: u8 = 2;
        const PIECE_LEN: u8 = 64;
        const FILE_LEN: i64 = PIECE_LEN as i64 * NO_OF_PIECES as i64;
        let piece_template = (0..PIECE_LEN).collect::<Vec<u8>>();
        let mut original_data = [0; NO_OF_PIECES as usize * PIECE_LEN as usize];
        let mut count = 0;
        let mut idx = 0;
        while idx < original_data.len() {
            let piece = piece_template[..]
                .iter()
                .map(|val| val + count * 10)
                .collect::<Vec<u8>>();
            original_data[idx..idx + PIECE_LEN as usize].copy_from_slice(&piece);
            count += 1;
            idx += PIECE_LEN as usize;
        }

        let mut piece_hashes = Vec::new();
        for idx in 0..NO_OF_PIECES {
            let piece = &original_data
                [idx as usize * PIECE_LEN as usize..(idx as usize + 1) * PIECE_LEN as usize];
            let hash = sha1_smol::Sha1::from(piece).digest().bytes();
            piece_hashes.append(&mut hash.to_vec());
        }

        let ip = "127.0.0.1";
        let port = 12346;
        let addr = format!("{}:{}", ip, port);
        let listener = std::net::TcpListener::bind(addr).unwrap();
        let peers = vec![Peer {
            ip: std::net::Ipv4Addr::new(127, 0, 0, 1),
            port,
        }];

        let announce = b"hello";
        let mut metainfo_map = HashMap::new();
        metainfo_map.insert(
            b"announce".to_vec(),
            BencodeType2::ByteString(announce.to_vec()),
        );

        let name = b"hello";
        let mut info_map = HashMap::new();
        info_map.insert(b"name".to_vec(), BencodeType2::ByteString(name.to_vec()));
        info_map.insert(b"length".to_vec(), BencodeType2::Integer(FILE_LEN));
        info_map.insert(
            b"piece length".to_vec(),
            BencodeType2::Integer(PIECE_LEN as i64),
        );
        info_map.insert(b"pieces".to_vec(), BencodeType2::ByteString(piece_hashes));
        metainfo_map.insert(b"info".to_vec(), BencodeType2::Dict(info_map));
        let data = BencodeType2::Dict(metainfo_map);
        let metainfo = Metainfo::new(data).unwrap();
        let torrent = Torrent::new(metainfo, peers);

        let their_peer_id = "-DEF123-efgh12345678";
        let response_handshake = Handshake::new(
            PSTR.to_string(),
            torrent.info_hash.clone(),
            their_peer_id.into(),
        );
        let len: u32 = 2;
        let id = 0x05;
        let payload = vec![0b11000000];
        let mut bitfield_message = u32::to_be_bytes(len).to_vec();
        bitfield_message.push(id);
        bitfield_message.append(&mut payload.clone());

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

        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut handshake_buf = [0; HANDSHAKE_BYTES_LEN];
            let mut read_unchoke_buf = [0; 5];
            let mut read_interested_buf = [0; 5];
            let mut piece_request_buf = [0; 29];
            let (mut socket, _) = listener.accept().unwrap();
            socket.read_exact(&mut handshake_buf).unwrap();
            socket.write_all(&response_handshake.serialise()).unwrap();
            socket.write_all(&bitfield_message).unwrap();
            socket.read_exact(&mut read_unchoke_buf).unwrap();
            socket.read_exact(&mut read_interested_buf).unwrap();
            socket.write_all(&Message::Unchoke.serialise()).unwrap();
            socket.read_exact(&mut piece_request_buf).unwrap();
            socket.read_exact(&mut piece_request_buf).unwrap();
            socket
                .write_all(&piece_zero_responses[0].clone().serialise())
                .unwrap();
            socket
                .write_all(&piece_zero_responses[1].clone().serialise())
                .unwrap();
            socket.read_exact(&mut piece_request_buf).unwrap();
            socket.read_exact(&mut piece_request_buf).unwrap();
            socket
                .write_all(&piece_one_responses[0].clone().serialise())
                .unwrap();
            socket
                .write_all(&piece_one_responses[1].clone().serialise())
                .unwrap();
            while rx.recv().is_ok() {}
        });

        let res = download(torrent).await;
        tx.send(Some(true)).unwrap();
        assert_eq!(res, original_data);
    }
}
