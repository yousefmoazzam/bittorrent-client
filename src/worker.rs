use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc::Sender;
use tracing::{debug, warn};

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
            if !self.client.bitfield.has_piece(work.index as usize) {
                self.work_queue.enqueue(work);
                tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
                continue;
            }
            let index = work.index;
            let buf = self.download_piece(&work).await?;
            if self.check_integrity(&work, &buf) {
                self.client
                    .send(Message::Have(index.try_into().unwrap()))
                    .await
                    .unwrap();
                self.piece_sender.send(Piece { index, buf }).await.unwrap();
            } else {
                self.work_queue.enqueue(work);
            }
        }

        Ok(())
    }

    /// Download piece described by [`Work`]
    async fn download_piece(&mut self, work: &Work) -> Result<Vec<u8>, std::io::Error> {
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

                    if let Err(e) = self
                        .client
                        .send(Message::Request {
                            index: work.index,
                            begin: block_index,
                            length: block_size,
                        })
                        .await
                    {
                        warn!("Received error when sending request message: {}", e)
                    };
                    block_index += block_size;
                }
            }

            match self.client.receive().await {
                Err(e) => return Err(e),
                Ok(message) => {
                    debug!("Received message: {}", message);
                    if let Message::Piece { begin, block, .. } = message {
                        buf[begin as usize..begin as usize + block.len()].copy_from_slice(&block);
                        bytes_downloaded += u64::try_from(block.len()).unwrap();
                    }
                }
            }
        }
        Ok(buf)
    }

    /// Check SHA1 hash of downloaded piece is as expected
    fn check_integrity(&self, work: &Work, piece: &[u8]) -> bool {
        let hash = sha1_smol::Sha1::from(piece).digest().bytes();
        hash == *work.hash
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
        let client = Client::new(mock_socket, info_hash).await.unwrap();
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
        let payload = vec![0b11000000];
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
        builder.write(&Message::Have(0).serialise());
        for request in piece_one_requests {
            builder = builder.write(&request.serialise());
        }
        for response in piece_one_responses {
            builder = builder.read(&response.serialise());
        }
        builder.write(&Message::Have(1).serialise());
        let socket = builder.build();
        let client = Client::new(socket, info_hash).await.unwrap();

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
        let payload = vec![0b11000000];
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
        builder.write(&Message::Have(0).serialise());
        for request in piece_one_requests {
            builder = builder.write(&request.serialise());
        }
        for response in piece_one_responses {
            builder = builder.read(&response.serialise());
        }
        builder.write(&Message::Have(1).serialise());
        let socket = builder.build();
        let client = Client::new(socket, info_hash).await.unwrap();

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
    async fn integrity_check_failure_causes_rerequest_of_piece() {
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
        let payload = vec![0b11000000];
        let mut bitfield_message = u32::to_be_bytes(len).to_vec();
        bitfield_message.push(id);
        bitfield_message.append(&mut payload.clone());

        // Setup mock socket with expected block requests/responses (along with all other
        // preliminary interactions, such as a sucessful handshake)
        let incorrect_block_0_piece_0 = [6; PIECE_LEN as usize / 2];
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
        let piece_zero_responses_incorrect_block_0 = [
            Message::Piece {
                index: 0,
                begin: 0,
                block: incorrect_block_0_piece_0.to_vec(),
            },
            Message::Piece {
                index: 0,
                begin: PIECE_LEN as u64 / 2,
                block: original_data[PIECE_LEN as usize / 2..PIECE_LEN as usize].to_vec(),
            },
        ];
        let piece_zero_responses_correct_block_0 = [
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
        for request in piece_zero_requests.clone() {
            builder = builder.write(&request.serialise());
        }
        for response in piece_zero_responses_incorrect_block_0 {
            builder = builder.read(&response.serialise());
        }
        for request in piece_zero_requests {
            builder = builder.write(&request.serialise());
        }
        for response in piece_zero_responses_correct_block_0 {
            builder = builder.read(&response.serialise());
        }
        builder.write(&Message::Have(0).serialise());
        for request in piece_one_requests {
            builder = builder.write(&request.serialise());
        }
        for response in piece_one_responses {
            builder = builder.read(&response.serialise());
        }
        builder.write(&Message::Have(1).serialise());
        let socket = builder.build();
        let client = Client::new(socket, info_hash).await.unwrap();

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
    async fn two_workers_download_the_two_pieces_from_their_respective_peer() {
        // Setup file data
        const NO_OF_PIECES: u8 = 4;
        const PIECE_LEN: u8 = 64;
        let original_data = (0..=255).collect::<Vec<u8>>();

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
        let queue_handle = queue.clone();

        // Setup info for preliminary client interactions with peers
        let info_hash = (0x00..0x14).collect::<Vec<_>>();
        let message_len: u32 = 2;
        let message_id = 0x05;

        let peer_0_id = "-DEF123-efgh12345678";
        let peer_0_initial_handshake =
            Handshake::new(PSTR.to_string(), info_hash.clone(), PEER_ID.into());
        let peer_0_response_handshake =
            Handshake::new(PSTR.to_string(), info_hash.clone(), peer_0_id.into());
        let peer_0_bitfield_payload = vec![0b10100000];
        let mut peer_0_bitfield_message = u32::to_be_bytes(message_len).to_vec();
        peer_0_bitfield_message.push(message_id);
        peer_0_bitfield_message.append(&mut peer_0_bitfield_payload.clone());

        let peer_1_id = "-HIJ123-ijkl12345678";
        let peer_1_initial_handshake =
            Handshake::new(PSTR.to_string(), info_hash.clone(), PEER_ID.into());
        let peer_1_response_handshake =
            Handshake::new(PSTR.to_string(), info_hash.clone(), peer_1_id.into());
        let peer_1_bitfield_payload = vec![0b01010000];
        let mut peer_1_bitfield_message = u32::to_be_bytes(message_len).to_vec();
        peer_1_bitfield_message.push(message_id);
        peer_1_bitfield_message.append(&mut peer_1_bitfield_payload.clone());

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
        let piece_two_requests = [
            Message::Request {
                index: 2,
                begin: 0,
                length: PIECE_LEN as u64 / 2,
            },
            Message::Request {
                index: 2,
                begin: PIECE_LEN as u64 / 2,
                length: PIECE_LEN as u64 / 2,
            },
        ];
        let piece_two_responses = [
            Message::Piece {
                index: 2,
                begin: 0,
                block: original_data
                    [2 * PIECE_LEN as usize..(2 * PIECE_LEN + PIECE_LEN / 2) as usize]
                    .to_vec(),
            },
            Message::Piece {
                index: 2,
                begin: PIECE_LEN as u64 / 2,
                block: original_data
                    [(2 * PIECE_LEN + PIECE_LEN / 2) as usize..3 * PIECE_LEN as usize]
                    .to_vec(),
            },
        ];
        let piece_three_requests = [
            Message::Request {
                index: 3,
                begin: 0,
                length: PIECE_LEN as u64 / 2,
            },
            Message::Request {
                index: 3,
                begin: PIECE_LEN as u64 / 2,
                length: PIECE_LEN as u64 / 2,
            },
        ];
        let piece_three_responses = [
            Message::Piece {
                index: 3,
                begin: 0,
                block: original_data
                    [3 * PIECE_LEN as usize..(3 * PIECE_LEN + PIECE_LEN / 2) as usize]
                    .to_vec(),
            },
            Message::Piece {
                index: 3,
                begin: PIECE_LEN as u64 / 2,
                block: original_data
                    [(3 * PIECE_LEN + PIECE_LEN / 2) as usize..4 * PIECE_LEN as usize]
                    .to_vec(),
            },
        ];

        // Define mock socket for peer 0
        let mut peer_0_builder = tokio_test::io::Builder::new();
        let mut peer_0_builder = peer_0_builder
            .write(&peer_0_initial_handshake.serialise())
            .read(&peer_0_response_handshake.serialise())
            .read(&peer_0_bitfield_message)
            .write(&Message::Unchoke.serialise())
            .write(&Message::Interested.serialise())
            .read(&Message::Unchoke.serialise());
        for request in piece_zero_requests {
            peer_0_builder = peer_0_builder.write(&request.serialise());
        }
        for response in piece_zero_responses {
            peer_0_builder = peer_0_builder.read(&response.serialise());
        }
        peer_0_builder.write(&Message::Have(0).serialise());
        for request in piece_two_requests {
            peer_0_builder = peer_0_builder.write(&request.serialise());
        }
        for response in piece_two_responses {
            peer_0_builder = peer_0_builder.read(&response.serialise());
        }
        peer_0_builder.write(&Message::Have(2).serialise());
        let peer_0_socket = peer_0_builder.build();
        let peer_0_client = Client::new(peer_0_socket, info_hash.clone()).await.unwrap();

        // Define mock socket for peer 1
        let mut peer_1_builder = tokio_test::io::Builder::new();
        let mut peer_1_builder = peer_1_builder
            .write(&peer_1_initial_handshake.serialise())
            .read(&peer_1_response_handshake.serialise())
            .read(&peer_1_bitfield_message)
            .write(&Message::Unchoke.serialise())
            .write(&Message::Interested.serialise())
            .read(&Message::Unchoke.serialise());
        for request in piece_one_requests {
            peer_1_builder = peer_1_builder.write(&request.serialise());
        }
        for response in piece_one_responses {
            peer_1_builder = peer_1_builder.read(&response.serialise());
        }
        peer_1_builder.write(&Message::Have(1).serialise());
        for request in piece_three_requests {
            peer_1_builder = peer_1_builder.write(&request.serialise());
        }
        for response in piece_three_responses {
            peer_1_builder = peer_1_builder.read(&response.serialise());
        }
        peer_1_builder.write(&Message::Have(3).serialise());
        let peer_1_socket = peer_1_builder.build();
        let peer_1_client = Client::new(peer_1_socket, info_hash).await.unwrap();

        // Setup channel that downloaded pieces get sent through
        let mut receiver_buf = [0; NO_OF_PIECES as usize * PIECE_LEN as usize];
        let (tx, rx) = tokio::sync::mpsc::channel(NO_OF_PIECES as usize);
        let tx1 = tx.clone();

        // Spawn tokio tasks to run workers
        let mut peer_0_worker = Worker::new(peer_0_client, tx, queue);
        tokio::spawn(async move {
            peer_0_worker.download().await.unwrap();
        });
        let mut peer_1_worker = Worker::new(peer_1_client, tx1, queue_handle);
        tokio::spawn(async move {
            peer_1_worker.download().await.unwrap();
        });

        // Run piece-receiver
        crate::piece::receiver(&mut receiver_buf, PIECE_LEN as usize, rx).await;

        assert_eq!(receiver_buf, &original_data[..]);
    }

    #[tokio::test]
    async fn send_have_message_to_peer_if_piece_downloaded_and_passed_integrity_check() {
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
        let payload = vec![0b11000000];
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
        builder.write(&Message::Have(0).serialise());
        for request in piece_one_requests {
            builder = builder.write(&request.serialise());
        }
        for response in piece_one_responses {
            builder = builder.read(&response.serialise());
        }
        builder.write(&Message::Have(1).serialise());
        let socket = builder.build();
        let client = Client::new(socket, info_hash).await.unwrap();

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
    async fn worker_propagates_unexpected_eof_error_if_peer_unexpectedly_closes_connection() {
        // Setup file data
        const NO_OF_PIECES: u8 = 1;
        const PIECE_LEN: u8 = 64;
        let original_data = (0..PIECE_LEN).collect::<Vec<u8>>();

        // Calculate SHA1 hash of the piece
        let piece_hash = sha1_smol::Sha1::from(original_data).digest().bytes();

        // Setup work queue
        let work = vec![Work {
            index: 0,
            length: PIECE_LEN as u64,
            hash: piece_hash.to_vec(),
        }];
        let queue = SharedQueue::new(work);

        // Setup info for preliminary client interactions with peer
        let info_hash = (0x00..0x14).collect::<Vec<_>>();
        let their_peer_id = "-DEF123-efgh12345678";
        let initial_handshake = Handshake::new(PSTR.to_string(), info_hash.clone(), PEER_ID.into());
        let response_handshake =
            Handshake::new(PSTR.to_string(), info_hash.clone(), their_peer_id.into());
        let len: u32 = 2;
        let id = 0x05;
        let payload = vec![0b10000000];
        let mut bitfield_message = u32::to_be_bytes(len).to_vec();
        bitfield_message.push(id);
        bitfield_message.append(&mut payload.clone());

        // Setup mock socket to produce an unexpected EOF error to mimic the peer unexpectedly
        // closing their side of the connection (after all other preliminary interactions, such as
        // a sucessful handshake)
        let socket = tokio_test::io::Builder::new()
            .write(&initial_handshake.serialise())
            .read(&response_handshake.serialise())
            .read(&bitfield_message)
            .write(&Message::Unchoke.serialise())
            .write(&Message::Interested.serialise())
            .read_error(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, ""))
            .build();
        let client = Client::new(socket, info_hash).await.unwrap();

        let mut receiver_buf = [0; PIECE_LEN as usize];
        let (tx, rx) = tokio::sync::mpsc::channel(NO_OF_PIECES as usize);
        let mut worker = Worker::new(client, tx, queue);
        let ret = tokio::spawn(async move { worker.download().await })
            .await
            .unwrap();
        crate::piece::receiver(&mut receiver_buf, PIECE_LEN as usize, rx).await;
        assert!(ret.is_err_and(|err| matches!(err.kind(), std::io::ErrorKind::UnexpectedEof)));
    }
}
