use tokio::sync::mpsc::Sender;

use crate::client::Client;
use crate::piece::Piece;
use crate::torrent::Torrent;
use crate::tracker::Peer;
use crate::work::{SharedQueue, Work};
use crate::worker::Worker;

/// Download file
pub async fn download(torrent: Torrent) -> Vec<u8> {
    let mut piece_bytes = Vec::new();
    for piece in torrent.metainfo.info.pieces() {
        let mut bytes = [""; 20];
        for (idx, char_idx) in (0..40).step_by(2).enumerate() {
            bytes[idx] = &piece[char_idx..char_idx + 2];
        }
        piece_bytes.push(bytes);
    }
    let no_of_pieces = piece_bytes.len();
    let piece_hashes = piece_bytes
        .into_iter()
        .map(str_arr_to_u8_arr)
        .collect::<Vec<_>>();
    let work = piece_hashes
        .into_iter()
        .enumerate()
        .map(|(idx, hash)| Work {
            index: u64::try_from(idx).unwrap(),
            length: u64::try_from(torrent.metainfo.info.piece_length).unwrap(),
            hash: hash.to_vec(),
        })
        .collect::<Vec<_>>();
    let queue = SharedQueue::new(work);
    let (tx, rx) = tokio::sync::mpsc::channel(no_of_pieces);

    println!("No of peers is {}", torrent.peers.len());

    for peer in torrent.peers {
        let info_hash = torrent.info_hash.clone();
        let queue = queue.clone();
        let tx = tx.clone();
        tokio::spawn(async move {
            //process(info_hash, peer, queue, tx).await;
            let addr = format!("{}:{}", peer.ip, peer.port);
            println!("CLIENT: Before trying to connect to peer");
            let socket = tokio::net::TcpStream::connect(addr).await.unwrap();
            println!("CLIENT: Before creating a new client");
            let client = Client::new(socket, info_hash).await.unwrap();
            println!("CLIENT: Before creating a new worker");
            let mut worker = Worker::new(client, tx, queue);
            println!("CLIENT: Before starting worker download");
            worker.download().await.unwrap();
        });
    }

    let mut buf = vec![0; torrent.metainfo.info.length];
    crate::piece::receiver(&mut buf, torrent.metainfo.info.piece_length, rx).await;
    buf
}

fn str_arr_to_u8_arr(arr: [&str; 20]) -> [u8; 20] {
    arr.map(|val| u8::from_str_radix(val, 16).unwrap())
}

//async fn process(info_hash: Vec<u8>, peer: Peer, queue: SharedQueue, tx: Sender<Piece>) {
//    let addr = format!("{}:{}", peer.ip, peer.port);
//    println!("CLIENT: Before trying to connect to peer");
//    let socket = tokio::net::TcpStream::connect(addr).await.unwrap();
//    println!("CLIENT: Before creating a new client");
//    let client = Client::new(socket, info_hash).await.unwrap();
//    println!("CLIENT: Before creating a new worker");
//    let mut worker = Worker::new(client, tx, queue);
//    println!("CLIENT: Before starting worker download");
//    worker.download().await.unwrap();
//}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::io::{Read, Write};

    use tokio::io::AsyncReadExt;
    use tokio::net::TcpListener;

    use crate::handshake::Handshake;
    use crate::message::Message;
    use crate::metainfo::Metainfo;
    use crate::tracker::Peer;
    use crate::BencodeType;
    use crate::{PEER_ID, PSTR};

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
        //let mut pieces = Vec::new();
        while idx < original_data.len() {
            let piece = piece_template[..]
                .iter()
                .map(|val| val + count * 10)
                .collect::<Vec<u8>>();
            original_data[idx..idx + PIECE_LEN as usize].copy_from_slice(&piece);
            //pieces.push(piece);
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
        let port = 12346;
        let addr = format!("{}:{}", ip, port);
        let listener = std::net::TcpListener::bind(addr).unwrap();
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
        println!(
            "bitfield_message to send from peer to client is {:?}",
            bitfield_message
        );

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

        let mock_peer_socket_handle = std::thread::spawn(move || {
            let mut throwaway_buf = [0; 1024];
            println!("PEER: Before accepting socket connection");
            let (mut socket, _) = listener.accept().unwrap();
            println!("PEER: After accepting socket connection");
            println!("PEER: Before reading initial handshake");
            let bytes_read = socket.read(&mut throwaway_buf).unwrap();
            println!(
                "PEER: Initial handshake data from client apparently was: {:?}",
                &throwaway_buf[..bytes_read]
            );
            println!("PEER: Before writing handshake response");
            socket.write_all(&response_handshake.serialise()).unwrap();
            println!("PEER: Before writing bitfield message");
            socket.write_all(&bitfield_message).unwrap();
            println!("PEER: Before reading piece 0 block 0 request");
            let _ = socket.read(&mut throwaway_buf).unwrap();
            println!("PEER: Before reading piece 0 block 1 request");
            let _ = socket.read(&mut throwaway_buf).unwrap();
            println!("PEER: Before writing piece 0 block 0 response");
            socket
                .write_all(&piece_zero_responses[0].clone().serialise())
                .unwrap();
            socket
                .write_all(&piece_zero_responses[1].clone().serialise())
                .unwrap();
            println!("After writing piece 0 block 0 response");
            let _ = socket.read(&mut throwaway_buf).unwrap();
            println!("After reading piece 1 block 0 request");
            println!("After reading piece 1 block 1 request");
            let _ = socket.read(&mut throwaway_buf).unwrap();
            socket
                .write_all(&piece_one_responses[0].clone().serialise())
                .unwrap();
            println!("After writing piece 1 block 0 response");
            socket
                .write_all(&piece_one_responses[1].clone().serialise())
                .unwrap();
            println!("After writing piece 1 block 1 response");
        });

        let res = download(torrent).await;
        //mock_peer_socket_handle.await.unwrap();
        mock_peer_socket_handle.join().unwrap();
        assert_eq!(res, original_data);
    }
}
