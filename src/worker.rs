use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc::Sender;

use crate::client::Client;
use crate::message::Message;
use crate::piece::Piece;
use crate::work::{SharedQueue, Work};

const DEFAULT_BLOCK_SIZE: u64 = 2u64.pow(14);

/// Piece download worker
pub struct Worker<T: AsyncRead + AsyncWrite + Unpin> {
    client: Client<T>,
    piece_sender: Sender<Piece>,
    work_queue: SharedQueue,
}

impl<T: AsyncRead + AsyncWrite + Unpin> Worker<T> {
    pub fn new(client: Client<T>, tx: Sender<Piece>, queue: SharedQueue) -> Worker<T> {
        Worker {
            client,
            piece_sender: tx,
            work_queue: queue,
        }
    }

    /// Download pieces from connected peer
    pub async fn download(&mut self) -> std::io::Result<()> {
        self.client.send(Message::Unchoke).await?;
        self.client.send(Message::Interested).await?;

        while let Some(work) = self.work_queue.dequeue() {
            let index = work.index;
            let buf = self.download_piece(work).await;
            self.piece_sender.send(Piece { index, buf }).await.unwrap();
        }

        Ok(())
    }

    /// Download piece described by [`Work`]
    async fn download_piece(&mut self, work: Work) -> Vec<u8> {
        let mut buf = vec![0; work.length as usize];
        let mut bytes_downloaded = 0;
        let mut block_index = 0;

        while bytes_downloaded < work.length {
            if !self.client.choked {
                while block_index < work.length {
                    let block_size = if DEFAULT_BLOCK_SIZE >= work.length {
                        work.length / 2
                    } else if block_index + DEFAULT_BLOCK_SIZE >= work.length {
                        work.length - block_index
                    } else {
                        DEFAULT_BLOCK_SIZE
                    };

                    self.client
                        .send(Message::Request {
                            index: work.index,
                            begin: block_index,
                            length: block_size,
                        })
                        .await
                        .unwrap();
                    block_index += block_size;
                }
            }

            let message = self.client.receive().await.unwrap();
            if let Message::Piece { begin, block, .. } = message {
                buf[begin as usize..begin as usize + block.len()].copy_from_slice(&block);
                bytes_downloaded += u64::try_from(block.len()).unwrap();
            }
        }
        buf
    }
}

#[cfg(test)]
mod tests {
    use crate::handshake::Handshake;
    use crate::{PEER_ID, PSTR};

    use super::*;

    #[tokio::test]
    async fn download_sends_unchoke_and_interested_messages_to_peer() {
        let info_hash = (0x00..0x14).collect::<Vec<_>>();
        let their_peer_id = "-DEF123-efgh12345678";

        let initial_handshake = Handshake::new(PSTR.to_string(), info_hash.clone(), PEER_ID.into());
        let response_handshake =
            Handshake::new(PSTR.to_string(), info_hash.clone(), their_peer_id.into());
        let len: u32 = 2;
        let id = 0x05;
        let payload = vec![0x10];
        let mut bitfield_message = u32::to_be_bytes(len).to_vec();
        bitfield_message.push(id);
        bitfield_message.append(&mut payload.clone());

        let mock_socket = tokio_test::io::Builder::new()
            .write(&initial_handshake.serialise())
            .read(&response_handshake.serialise())
            .read(&bitfield_message)
            .write(&Message::Unchoke.serialise())
            .write(&Message::Interested.serialise())
            .build();
        let client = Client::new(mock_socket, info_hash, their_peer_id)
            .await
            .unwrap();
        let (tx, _) = tokio::sync::mpsc::channel(1);
        let mut worker = Worker::new(client, tx, SharedQueue::new(vec![]));
        let _ = worker.download().await;
    }

    #[tokio::test]
    async fn single_worker_downloads_all_two_pieces_from_peer() {
        // Setup file data
        const NO_OF_PIECES: u8 = 2;
        const PIECE_LEN: u8 = 64;
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

        // Calculate SHA1 hashes of the pieces
        let mut piece_hashes = Vec::new();
        for idx in 0..NO_OF_PIECES {
            let piece = &original_data
                [idx as usize * PIECE_LEN as usize..(idx as usize + 1) * PIECE_LEN as usize];
            let hash = sha1_smol::Sha1::from(piece).digest().bytes();
            piece_hashes.push(hash);
        }

        // Setup work queue
        let work = (0..NO_OF_PIECES as u64)
            .map(|index| Work {
                index,
                length: PIECE_LEN as u64,
                hash: piece_hashes[index as usize].to_vec(),
            })
            .collect::<Vec<_>>();
        let queue = SharedQueue::new(work);

        // Setup info for preliminary client interactions with peer
        let info_hash = (0x00..0x14).collect::<Vec<_>>();
        let their_peer_id = "-DEF123-efgh12345678";
        let initial_handshake = Handshake::new(PSTR.to_string(), info_hash.clone(), PEER_ID.into());
        let response_handshake =
            Handshake::new(PSTR.to_string(), info_hash.clone(), their_peer_id.into());
        let len: u32 = 2;
        let id = 0x05;
        let payload = vec![0b00000011];
        let mut bitfield_message = u32::to_be_bytes(len).to_vec();
        bitfield_message.push(id);
        bitfield_message.append(&mut payload.clone());

        // Setup mock socket with expected block requests/responses (along with all other
        // preliminary interactions, such as a sucessful handshake)
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
        let mut builder = tokio_test::io::Builder::new();
        let mut builder = builder
            .write(&initial_handshake.serialise())
            .read(&response_handshake.serialise())
            .read(&bitfield_message)
            .write(&Message::Unchoke.serialise())
            .write(&Message::Interested.serialise())
            .read(&Message::Unchoke.serialise());
        for request in piece_zero_requests {
            builder = builder.write(&request.serialise());
        }
        for response in piece_zero_responses {
            builder = builder.read(&response.serialise());
        }
        for request in piece_one_requests {
            builder = builder.write(&request.serialise());
        }
        for response in piece_one_responses {
            builder = builder.read(&response.serialise());
        }
        let socket = builder.build();
        let client = Client::new(socket, info_hash, their_peer_id).await.unwrap();

        // Setup channel that downloaded pieces get sent through
        let mut receiver_buf = [0; NO_OF_PIECES as usize * PIECE_LEN as usize];
        let (tx, rx) = tokio::sync::mpsc::channel(NO_OF_PIECES as usize);

        // Spawn tokio task to run worker
        let mut worker = Worker::new(client, tx, queue);
        tokio::spawn(async move {
            worker.download().await.unwrap();
        });

        // Run piece-receiver
        crate::piece::receiver(&mut receiver_buf, PIECE_LEN as usize, rx).await;

        assert_eq!(receiver_buf, original_data);
    }

    #[tokio::test]
    async fn single_worker_downloads_pieces_larger_than_default_block_size_but_not_divisible_by_it()
    {
        // Setup file data
        const NO_OF_PIECES: u8 = 2;
        const PIECE_LEN: u64 = 18_000;
        let piece_template = [5; PIECE_LEN as usize];
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

        // Calculate SHA1 hashes of the pieces
        let mut piece_hashes = Vec::new();
        for idx in 0..NO_OF_PIECES {
            let piece = &original_data
                [idx as usize * PIECE_LEN as usize..(idx as usize + 1) * PIECE_LEN as usize];
            let hash = sha1_smol::Sha1::from(piece).digest().bytes();
            piece_hashes.push(hash);
        }

        // Setup work queue
        let work = (0..NO_OF_PIECES as u64)
            .map(|index| Work {
                index,
                length: PIECE_LEN,
                hash: piece_hashes[index as usize].to_vec(),
            })
            .collect::<Vec<_>>();
        let queue = SharedQueue::new(work);

        // Setup info for preliminary client interactions with peer
        let info_hash = (0x00..0x14).collect::<Vec<_>>();
        let their_peer_id = "-DEF123-efgh12345678";
        let initial_handshake = Handshake::new(PSTR.to_string(), info_hash.clone(), PEER_ID.into());
        let response_handshake =
            Handshake::new(PSTR.to_string(), info_hash.clone(), their_peer_id.into());
        let len: u32 = 2;
        let id = 0x05;
        let payload = vec![0b00000011];
        let mut bitfield_message = u32::to_be_bytes(len).to_vec();
        bitfield_message.push(id);
        bitfield_message.append(&mut payload.clone());

        // Setup mock socket with expected block requests/responses (along with all other
        // preliminary interactions, such as a sucessful handshake)
        const SECOND_BLOCK_LEN: u64 = PIECE_LEN - DEFAULT_BLOCK_SIZE;
        let piece_zero_requests = [
            Message::Request {
                index: 0,
                begin: 0,
                length: DEFAULT_BLOCK_SIZE,
            },
            Message::Request {
                index: 0,
                begin: DEFAULT_BLOCK_SIZE,
                length: SECOND_BLOCK_LEN,
            },
        ];
        let piece_zero_responses = [
            Message::Piece {
                index: 0,
                begin: 0,
                block: original_data[..DEFAULT_BLOCK_SIZE as usize].to_vec(),
            },
            Message::Piece {
                index: 0,
                begin: DEFAULT_BLOCK_SIZE,
                block: original_data
                    [DEFAULT_BLOCK_SIZE as usize..(DEFAULT_BLOCK_SIZE + SECOND_BLOCK_LEN) as usize]
                    .to_vec(),
            },
        ];
        let piece_one_requests = [
            Message::Request {
                index: 1,
                begin: 0,
                length: DEFAULT_BLOCK_SIZE,
            },
            Message::Request {
                index: 1,
                begin: DEFAULT_BLOCK_SIZE,
                length: SECOND_BLOCK_LEN,
            },
        ];
        let piece_one_responses = [
            Message::Piece {
                index: 1,
                begin: 0,
                block: original_data[(DEFAULT_BLOCK_SIZE + SECOND_BLOCK_LEN) as usize
                    ..(2 * DEFAULT_BLOCK_SIZE + SECOND_BLOCK_LEN) as usize]
                    .to_vec(),
            },
            Message::Piece {
                index: 1,
                begin: DEFAULT_BLOCK_SIZE,
                block: original_data[(2 * DEFAULT_BLOCK_SIZE + SECOND_BLOCK_LEN) as usize..]
                    .to_vec(),
            },
        ];
        let mut builder = tokio_test::io::Builder::new();
        let mut builder = builder
            .write(&initial_handshake.serialise())
            .read(&response_handshake.serialise())
            .read(&bitfield_message)
            .write(&Message::Unchoke.serialise())
            .write(&Message::Interested.serialise())
            .read(&Message::Unchoke.serialise());
        for request in piece_zero_requests {
            builder = builder.write(&request.serialise());
        }
        for response in piece_zero_responses {
            builder = builder.read(&response.serialise());
        }
        for request in piece_one_requests {
            builder = builder.write(&request.serialise());
        }
        for response in piece_one_responses {
            builder = builder.read(&response.serialise());
        }
        let socket = builder.build();
        let client = Client::new(socket, info_hash, their_peer_id).await.unwrap();

        // Setup channel that downloaded pieces get sent through
        let mut receiver_buf = [0; NO_OF_PIECES as usize * PIECE_LEN as usize];
        let (tx, rx) = tokio::sync::mpsc::channel(NO_OF_PIECES as usize);

        // Spawn tokio task to run worker
        let mut worker = Worker::new(client, tx, queue);
        tokio::spawn(async move {
            worker.download().await.unwrap();
        });

        // Run piece-receiver
        crate::piece::receiver(&mut receiver_buf, PIECE_LEN as usize, rx).await;

        assert_eq!(receiver_buf, original_data);
    }
}
