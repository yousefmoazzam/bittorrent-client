use tokio::io::{AsyncRead, AsyncReadExt};
use tracing::warn;

const BITS_IN_BYTE: usize = 8;

/// Wrapper type for bitfield message payload
#[derive(Clone, Debug, PartialEq)]
pub struct Bitfield {
    data: Vec<u8>,
}

impl Bitfield {
    /// Create instance from bitfield message payload
    pub fn new(data: Vec<u8>) -> Bitfield {
        Bitfield { data }
    }

    /// Check if the bitfield contains the piece with the given index
    pub fn has_piece(&self, idx: usize) -> bool {
        let byte_index = idx / BITS_IN_BYTE;
        let offset = idx % BITS_IN_BYTE;
        let shifted_piece_bit = self.data[byte_index] >> (BITS_IN_BYTE - 1 - offset);
        shifted_piece_bit & 0b00000001 == 0b00000001
    }

    /// Set the bit in the bitfield that corresponds to the given piece index
    pub fn set_piece(&mut self, idx: usize) {
        let byte_index = idx / BITS_IN_BYTE;
        let offset = idx % BITS_IN_BYTE;
        let new_byte_value = 0b00000001 << (BITS_IN_BYTE - 1 - offset);
        self.data[byte_index] |= new_byte_value
    }
}

/// Peer message types
#[derive(Clone, Debug, PartialEq)]
pub enum Message {
    KeepAlive,
    Choke,
    Unchoke,
    Interested,
    NotInterested,
    /// Index of piece the downloader has completed and checked the hash of
    Have(u32),
    /// Describes which pieces (by index) the downloader has sent
    Bitfield(Bitfield),
    /// Request a subset of a piece (a block)
    Request {
        index: u32,
        begin: u32,
        length: u32,
    },
    /// Send a subset of a piece (a block)
    Piece {
        index: u32,
        begin: u32,
        block: Vec<u8>,
    },
    /// Cancel a request for a block
    Cancel {
        index: u32,
        begin: u32,
        length: u32,
    },
}

impl std::fmt::Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::KeepAlive => write!(f, "KeepAlive"),
            Self::Choke => write!(f, "Choke"),
            Self::Unchoke => write!(f, "Unchoke"),
            Self::Interested => write!(f, "Interested"),
            Self::NotInterested => write!(f, "NotInterested"),
            Self::Have(index) => write!(f, "Have({})", index),
            Self::Bitfield(Bitfield { data }) => write!(f, "Bitfield(len={})", data.len()),
            Self::Request {
                index,
                begin,
                length,
            } => write!(
                f,
                "Request(index={}, begin={}, len={})",
                index, begin, length
            ),
            Self::Piece {
                index,
                begin,
                block,
            } => write!(
                f,
                "Piece(index={}, begin={}, len={})",
                index,
                begin,
                block.len()
            ),
            Self::Cancel {
                index,
                begin,
                length,
            } => write!(
                f,
                "Cancel(index={}, begin={}, len={})",
                index, begin, length
            ),
        }
    }
}

impl Message {
    /// Deserialise raw bytes from socket to [`Message`]
    pub async fn deserialise<T>(socket: &mut T) -> std::io::Result<Message>
    where
        T: AsyncRead + Unpin,
    {
        let len = socket.read_u32().await.map_err(|err| match err.kind() {
            std::io::ErrorKind::UnexpectedEof => {
                warn!("Unexpected EOF when reading message length");
                err
            }
            _ => err,
        })?;
        if len == 0 {
            return Ok(Message::KeepAlive);
        }

        let id = socket.read_u8().await.map_err(|err| match err.kind() {
            std::io::ErrorKind::UnexpectedEof => {
                warn!("Unexpected EOF when reading message ID; len={}", len);
                err
            }
            _ => err,
        })?;

        if len == 1 {
            return match id {
                0x00 => Ok(Message::Choke),
                0x01 => Ok(Message::Unchoke),
                0x02 => Ok(Message::Interested),
                0x03 => Ok(Message::NotInterested),
                _ => Err(std::io::Error::other(format!(
                    "Invalid ID for single byte message: {}",
                    id
                ))),
            };
        }

        let mut bytes = vec![0; len as usize - 1];
        socket
            .read_exact(&mut bytes[..])
            .await
            .map_err(|err| match err.kind() {
                std::io::ErrorKind::UnexpectedEof => {
                    warn!(
                        "Unexpected EOF when reading message payload; len={}, id={}",
                        len, id
                    );
                    err
                }
                _ => err,
            })?;
        match id {
            0x04 => {
                let index = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                Ok(Message::Have(index))
            }
            0x05 => Ok(Message::Bitfield(Bitfield::new(bytes.to_vec()))),
            0x06 | 0x08 => {
                let index = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                let begin = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
                let length = u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
                if id == 0x06 {
                    Ok(Message::Request {
                        index,
                        begin,
                        length,
                    })
                } else {
                    Ok(Message::Cancel {
                        index,
                        begin,
                        length,
                    })
                }
            }
            0x07 => {
                let index = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                let begin = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
                let block = bytes[8..].to_vec();
                Ok(Message::Piece {
                    index,
                    begin,
                    block,
                })
            }
            _ => Err(std::io::Error::other(format!(
                "Invalid message ID for message containing non-zero payload: {}",
                id
            ))),
        }
    }

    /// Serialise [`Message`] to raw bytes
    pub fn serialise(self) -> Vec<u8> {
        match self {
            Message::KeepAlive => u32::to_be_bytes(0).to_vec(),
            Message::Choke => {
                let mut buf = u32::to_be_bytes(1).to_vec();
                buf.push(0);
                buf
            }
            Message::Unchoke => {
                let mut buf = u32::to_be_bytes(1).to_vec();
                buf.push(1);
                buf
            }
            Message::Interested => {
                let mut buf = u32::to_be_bytes(1).to_vec();
                buf.push(2);
                buf
            }
            Message::NotInterested => {
                let mut buf = u32::to_be_bytes(1).to_vec();
                buf.push(3);
                buf
            }
            Message::Have(index) => {
                let len = 1 + 4;
                let mut buf = u32::to_be_bytes(len).to_vec();
                buf.push(4);
                buf.append(&mut u32::to_be_bytes(index).to_vec());
                buf
            }
            Message::Bitfield(mut bitfield) => {
                let len = 1 + bitfield.data.len() as u32;
                let mut buf = u32::to_be_bytes(len).to_vec();
                buf.push(5);
                buf.append(&mut bitfield.data);
                buf
            }
            Message::Request {
                index,
                begin,
                length,
            } => {
                let len = 1 + (3 * 4);
                let mut buf = u32::to_be_bytes(len).to_vec();
                buf.push(6);
                buf.append(&mut u32::to_be_bytes(index).to_vec());
                buf.append(&mut u32::to_be_bytes(begin).to_vec());
                buf.append(&mut u32::to_be_bytes(length).to_vec());
                buf
            }
            Message::Piece {
                index,
                begin,
                mut block,
            } => {
                let len = 1 + 4 + 4 + block.len() as u32;
                let mut buf = u32::to_be_bytes(len).to_vec();
                buf.push(7);
                buf.append(&mut u32::to_be_bytes(index).to_vec());
                buf.append(&mut u32::to_be_bytes(begin).to_vec());
                buf.append(&mut block);
                buf
            }
            Message::Cancel {
                index,
                begin,
                length,
            } => {
                let len = 1 + (3 * 4);
                let mut buf = u32::to_be_bytes(len).to_vec();
                buf.push(8);
                buf.append(&mut u32::to_be_bytes(index).to_vec());
                buf.append(&mut u32::to_be_bytes(begin).to_vec());
                buf.append(&mut u32::to_be_bytes(length).to_vec());
                buf
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn parse_keep_alive_message() {
        let len: u32 = 0;
        let buf = u32::to_be_bytes(len).to_vec();
        let expected_message = Message::KeepAlive;
        let mut mock_socket = tokio_test::io::Builder::new().read(&buf[..]).build();
        let res = Message::deserialise(&mut mock_socket).await;
        assert!(res.is_ok_and(|message| message == expected_message))
    }

    #[tokio::test]
    async fn parse_choke_message() {
        let len: u32 = 1;
        let id = 0x00;
        let mut buf = u32::to_be_bytes(len).to_vec();
        buf.push(id);
        let expected_message = Message::Choke;
        let mut mock_socket = tokio_test::io::Builder::new().read(&buf[..]).build();
        let res = Message::deserialise(&mut mock_socket).await;
        assert!(res.is_ok_and(|message| message == expected_message));
    }

    #[tokio::test]
    async fn parse_unchoke_message() {
        let len: u32 = 1;
        let id = 0x01;
        let mut buf = u32::to_be_bytes(len).to_vec();
        buf.push(id);
        let expected_message = Message::Unchoke;
        let mut mock_socket = tokio_test::io::Builder::new().read(&buf[..]).build();
        let res = Message::deserialise(&mut mock_socket).await;
        assert!(res.is_ok_and(|message| message == expected_message));
    }

    #[tokio::test]
    async fn parse_interested_message() {
        let len: u32 = 1;
        let id = 0x02;
        let mut buf = u32::to_be_bytes(len).to_vec();
        buf.push(id);
        let expected_message = Message::Interested;
        let mut mock_socket = tokio_test::io::Builder::new().read(&buf[..]).build();
        let res = Message::deserialise(&mut mock_socket).await;
        assert!(res.is_ok_and(|message| message == expected_message));
    }

    #[tokio::test]
    async fn parse_not_interested_message() {
        let len: u32 = 1;
        let id = 0x03;
        let mut buf = u32::to_be_bytes(len).to_vec();
        buf.push(id);
        let expected_message = Message::NotInterested;
        let mut mock_socket = tokio_test::io::Builder::new().read(&buf[..]).build();
        let res = Message::deserialise(&mut mock_socket).await;
        assert!(res.is_ok_and(|message| message == expected_message));
    }

    #[tokio::test]
    async fn parse_have_message() {
        let len: u32 = 5;
        let id = 0x04;
        let index: u32 = 100;
        let mut buf = u32::to_be_bytes(len).to_vec();
        buf.push(id);
        buf.append(&mut u32::to_be_bytes(index).to_vec());
        let expected_message = Message::Have(index);
        let mut mock_socket = tokio_test::io::Builder::new().read(&buf[..]).build();
        let res = Message::deserialise(&mut mock_socket).await;
        assert!(res.is_ok_and(|message| message == expected_message));
    }

    #[tokio::test]
    async fn parse_bitfield_message() {
        let len: u32 = 2;
        let id = 0x05;
        let payload = vec![0x10];
        let mut buf = u32::to_be_bytes(len).to_vec();
        buf.push(id);
        buf.append(&mut payload.clone());
        let bitfield = Bitfield::new(payload);
        let expected_message = Message::Bitfield(bitfield);
        let mut mock_socket = tokio_test::io::Builder::new().read(&buf[..]).build();
        let res = Message::deserialise(&mut mock_socket).await;
        assert!(res.is_ok_and(|message| message == expected_message));
    }

    #[tokio::test]
    async fn parse_request_message() {
        let len: u32 = 13;
        let id = 0x06;
        let index: u32 = 30;
        let begin: u32 = 100;
        let length: u32 = 200;
        let mut buf = u32::to_be_bytes(len).to_vec();
        buf.push(id);
        buf.append(&mut u32::to_be_bytes(index).to_vec());
        buf.append(&mut u32::to_be_bytes(begin).to_vec());
        buf.append(&mut u32::to_be_bytes(length).to_vec());
        let expected_message = Message::Request {
            index,
            begin,
            length,
        };
        let mut mock_socket = tokio_test::io::Builder::new().read(&buf[..]).build();
        let res = Message::deserialise(&mut mock_socket).await;
        assert!(res.is_ok_and(|message| message == expected_message));
    }

    #[tokio::test]
    async fn parse_piece_message() {
        let id = 0x07;
        let index: u32 = 30;
        let begin: u32 = 100;
        let block = (0x00..0xFF).collect::<Vec<u8>>();
        let len: u32 = 1 + 4 + 4 + block.len() as u32;
        let mut buf = u32::to_be_bytes(len).to_vec();
        buf.push(id);
        buf.append(&mut u32::to_be_bytes(index).to_vec());
        buf.append(&mut u32::to_be_bytes(begin).to_vec());
        buf.append(&mut block.clone());
        let expected_message = Message::Piece {
            index,
            begin,
            block,
        };
        let mut mock_socket = tokio_test::io::Builder::new().read(&buf[..]).build();
        let res = Message::deserialise(&mut mock_socket).await;
        assert!(res.is_ok_and(|message| message == expected_message));
    }

    #[tokio::test]
    async fn parse_cancel_message() {
        let len: u32 = 13;
        let id = 0x08;
        let index: u32 = 30;
        let begin: u32 = 100;
        let length: u32 = 200;
        let mut buf = u32::to_be_bytes(len).to_vec();
        buf.push(id);
        buf.append(&mut u32::to_be_bytes(index).to_vec());
        buf.append(&mut u32::to_be_bytes(begin).to_vec());
        buf.append(&mut u32::to_be_bytes(length).to_vec());
        let expected_message = Message::Cancel {
            index,
            begin,
            length,
        };
        let mut mock_socket = tokio_test::io::Builder::new().read(&buf[..]).build();
        let res = Message::deserialise(&mut mock_socket).await;
        assert!(res.is_ok_and(|message| message == expected_message));
    }

    #[tokio::test]
    async fn return_error_if_single_byte_message_id_is_invalid() {
        let len: u32 = 1;
        let id = 0x04;
        let mut buf = u32::to_be_bytes(len).to_vec();
        buf.push(id);
        let mut mock_socket = tokio_test::io::Builder::new().read(&buf[..]).build();
        let res = Message::deserialise(&mut mock_socket).await;
        let expected_err_msg = "Invalid ID for single byte message: 4";
        assert!(res.is_err_and(|msg| msg.to_string() == expected_err_msg));
    }

    #[tokio::test]
    async fn return_error_if_non_zero_payload_message_id_is_invalid() {
        let len: u32 = 9;
        let id = 0x01;
        let mut buf = u32::to_be_bytes(len).to_vec();
        buf.push(id);
        buf.append(&mut [0; 8].to_vec());
        let mut mock_socket = tokio_test::io::Builder::new().read(&buf[..]).build();
        let res = Message::deserialise(&mut mock_socket).await;
        let expected_err_msg = "Invalid message ID for message containing non-zero payload: 1";
        assert!(res.is_err_and(|msg| msg.to_string() == expected_err_msg));
    }

    #[test]
    fn serialise_keep_alive_message() {
        let message = Message::KeepAlive;
        let len: u32 = 0;
        let expected_buf = u32::to_be_bytes(len).to_vec();
        assert_eq!(message.serialise(), expected_buf);
    }

    #[test]
    fn serialise_choke_message() {
        let message = Message::Choke;
        let len = 1;
        let id = 0;
        let mut expected_buf = u32::to_be_bytes(len).to_vec();
        expected_buf.push(id);
        assert_eq!(message.serialise(), expected_buf);
    }

    #[test]
    fn serialise_unchoke_message() {
        let message = Message::Unchoke;
        let len = 1;
        let id = 1;
        let mut expected_buf = u32::to_be_bytes(len).to_vec();
        expected_buf.push(id);
        assert_eq!(message.serialise(), expected_buf);
    }

    #[test]
    fn serialise_interested_message() {
        let message = Message::Interested;
        let len = 1;
        let id = 2;
        let mut expected_buf = u32::to_be_bytes(len).to_vec();
        expected_buf.push(id);
        assert_eq!(message.serialise(), expected_buf);
    }

    #[test]
    fn serialise_not_interested_message() {
        let message = Message::NotInterested;
        let len = 1;
        let id = 3;
        let mut expected_buf = u32::to_be_bytes(len).to_vec();
        expected_buf.push(id);
        assert_eq!(message.serialise(), expected_buf);
    }

    #[test]
    fn serialise_have_message() {
        let index = 10;
        let message = Message::Have(index);
        let len = 1 + 4;
        let id = 4;
        let mut expected_buf = u32::to_be_bytes(len).to_vec();
        expected_buf.push(id);
        expected_buf.append(&mut u32::to_be_bytes(index).to_vec());
        assert_eq!(message.serialise(), expected_buf);
    }

    #[test]
    fn serialise_bitfield_message() {
        let mut data = vec![0x03, 0x01];
        let len = 1 + data.len() as u32;
        let bitfield = Bitfield::new(data.clone());
        let id = 5;
        let message = Message::Bitfield(bitfield);
        let mut expected_buf = u32::to_be_bytes(len).to_vec();
        expected_buf.push(id);
        expected_buf.append(&mut data);
        assert_eq!(message.serialise(), expected_buf);
    }

    #[test]
    fn serialise_request_message() {
        let id = 6;
        let index = 2;
        let begin = 0;
        let block_len = 64;
        let len = 1 + (3 * 4);
        let mut expected_buf = u32::to_be_bytes(len).to_vec();
        expected_buf.push(id);
        expected_buf.append(&mut u32::to_be_bytes(index).to_vec());
        expected_buf.append(&mut u32::to_be_bytes(begin).to_vec());
        expected_buf.append(&mut u32::to_be_bytes(block_len).to_vec());
        let message = Message::Request {
            index,
            begin,
            length: block_len,
        };
        assert_eq!(message.serialise(), expected_buf);
    }

    #[test]
    fn serialise_piece_message() {
        let id = 7;
        let index = 30;
        let begin = 100;
        let block = (0x00..0xFF).collect::<Vec<u8>>();
        let len = 1 + 4 + 4 + block.len() as u32;
        let mut expected_buf = u32::to_be_bytes(len).to_vec();
        expected_buf.push(id);
        expected_buf.append(&mut u32::to_be_bytes(index).to_vec());
        expected_buf.append(&mut u32::to_be_bytes(begin).to_vec());
        expected_buf.append(&mut block.clone());
        let message = Message::Piece {
            index,
            begin,
            block,
        };
        assert_eq!(message.serialise(), expected_buf);
    }

    #[test]
    fn serialise_cancel_message() {
        let len = 1 + (3 * 4);
        let id = 8;
        let index = 30;
        let begin = 100;
        let length = 200;
        let mut expected_buf = u32::to_be_bytes(len).to_vec();
        expected_buf.push(id);
        expected_buf.append(&mut u32::to_be_bytes(index).to_vec());
        expected_buf.append(&mut u32::to_be_bytes(begin).to_vec());
        expected_buf.append(&mut u32::to_be_bytes(length).to_vec());
        let message = Message::Cancel {
            index,
            begin,
            length,
        };
        assert_eq!(message.serialise(), expected_buf);
    }

    #[test]
    fn bitfield_has_piece_returns_true_if_has_piece() {
        // Bits 2 and 14 (interpreting in big-endian/network order) mean pieces 2 and 14 are
        // available
        let data = vec![0b00100000, 0b00000010];
        let bitfield = Bitfield::new(data);
        assert!(bitfield.has_piece(2));
        assert!(bitfield.has_piece(14));
    }

    #[test]
    fn bitfield_set_piece_sets_correct_bit_in_bitfield() {
        let idx_to_set = 3;
        let data = vec![0b10000000, 0x00];
        let expected_bitfield = Bitfield::new(vec![0b10010000, 0x00]);
        let mut bitfield = Bitfield::new(data);
        bitfield.set_piece(idx_to_set);
        assert_eq!(bitfield, expected_bitfield);
    }
}
