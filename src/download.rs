use crate::client::Client;
use crate::piece::Piece;
use crate::torrent::Torrent;
use crate::tracker::Peer;
use crate::work::{SharedQueue, Work};
use crate::worker::Worker;
use tokio::sync::mpsc::Sender;
use tracing::{info, instrument, warn, Instrument};

/// Download file
pub async fn download(torrent: Torrent) -> Vec<u8> {
    let work = torrent
        .metainfo
        .info
        .pieces()
        .enumerate()
        .map(|(idx, hash)| Work {
            index: idx as u32,
            length: torrent.metainfo.info.piece_length as u32,
            hash: hash.to_vec(),
        })
        .collect::<Vec<_>>();
    let no_of_pieces = work.len();
    let queue = SharedQueue::new(work);
    let (tx, rx) = tokio::sync::mpsc::channel(no_of_pieces);
    let (completion_tx, completion_rx) = tokio::sync::watch::channel(false);

    for peer in torrent.peers {
        let info_hash = torrent.info_hash.clone();
        let queue = queue.clone();
        let tx = tx.clone();
        let completion_rx = completion_rx.clone();
        tokio::spawn(async move {
            process(info_hash, &peer, tx, completion_rx, queue).await;
        });
    }

    drop(tx);
    let mut buf = vec![0; torrent.metainfo.info.length];
    crate::piece::receiver(
        &mut buf,
        torrent.metainfo.info.piece_length,
        rx,
        no_of_pieces,
        completion_tx,
    )
    .await;
    buf
}

#[instrument(skip(info_hash, tx, completion_rx, queue))]
async fn process(
    info_hash: Vec<u8>,
    peer: &Peer,
    tx: Sender<Piece>,
    completion_rx: tokio::sync::watch::Receiver<bool>,
    queue: SharedQueue,
) {
    let addr = format!("{}:{}", peer.ip, peer.port);
    match tokio::net::TcpStream::connect(addr).await {
        Err(e) => {
            warn!("Unable to establish TCP connection: {}", e);
        }
        Ok(socket) => {
            info!("Established TCP connection");
            match Client::new(tokio::io::BufReader::new(socket), info_hash).await {
                Err(e) => warn!("Unable to establish peer protocol: {}", e),
                Ok(client) => {
                    info!("Established peer protocol");
                    async move {
                        let mut worker = Worker::new(client, tx, completion_rx, queue);
                        match worker.download().await {
                            Err(e) => warn!("Encountered error during download: {}", e),
                            Ok(_) => {
                                info!("Successful download")
                            }
                        };
                    }
                    .instrument(tracing::Span::current())
                    .await;
                }
            };
        }
    };
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::io::{Read, Write};

    use crate::handshake::Handshake;
    use crate::message::Message;
    use crate::metainfo::Metainfo;
    use crate::parse::BencodeType2;
    use crate::tracker::Peer;
    use crate::{HANDSHAKE_BYTES_LEN, PEER_ID, PSTR};

    use super::*;

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
                begin: PIECE_LEN as u32 / 2,
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
                begin: PIECE_LEN as u32 / 2,
                block: original_data
                    [PIECE_LEN as usize + PIECE_LEN as usize / 2..PIECE_LEN as usize * 2]
                    .to_vec(),
            },
        ];

        let initial_handshake =
            Handshake::new(PSTR.to_string(), torrent.info_hash.clone(), PEER_ID.into());
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut handshake_buf = [0; HANDSHAKE_BYTES_LEN];
            let mut read_unchoke_buf = [0; 5];
            let mut read_interested_buf = [0; 5];
            let mut piece_request_buf = [0; 17];
            let (mut socket, _) = listener.accept().unwrap();
            socket.read_exact(&mut handshake_buf).unwrap();
            assert_eq!(handshake_buf, &initial_handshake.serialise()[..]);
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
