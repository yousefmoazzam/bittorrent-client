use tokio::io::{AsyncRead, AsyncReadExt, BufReader};

const ID_INDEX: u8 = 4;

const BITS_IN_BYTE: usize = 8;

/// Wrapper type for bitfield message payload
#[derive(Debug, PartialEq)]
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
#[derive(Debug, PartialEq)]
pub enum Message {
    KeepAlive,
    Choke,
    Unchoke,
    Interested,
    NotInterested,
    /// Index of piece the downloader has completed and checked the hash of
    Have(u64),
    /// Describes which pieces (by index) the downloader has sent
    Bitfield(Bitfield),
    /// Request a subset of a piece (a block)
    Request {
        index: u64,
        begin: u64,
        length: u64,
    },
    /// Send a subset of a piece (a block)
    Piece {
        index: u64,
        begin: u64,
        block: Vec<u8>,
    },
    /// Cancel a request for a block
    Cancel {
        index: u64,
        begin: u64,
        length: u64,
    },
}

impl Message {
    /// Deserialise raw bytes from socket to [`Message`]
    pub async fn deserialise<T>(socket: T) -> std::io::Result<Message>
    where
        T: AsyncRead + Unpin,
    {
        let mut reader = BufReader::new(socket);
        let len = reader.read_u32().await?;
        if len == 0 {
            return Ok(Message::KeepAlive);
        }

        let id = reader.read_u8().await?;

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
        reader.read_exact(&mut bytes[..]).await?;
        match id {
            0x04 => {
                let index = u64::from_be_bytes([
                    bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
                ]);
                Ok(Message::Have(index))
            }
            0x05 => Ok(Message::Bitfield(Bitfield::new(bytes.to_vec()))),
            0x06 | 0x08 => {
                let index = u64::from_be_bytes([
                    bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
                ]);
                let begin = u64::from_be_bytes([
                    bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14],
                    bytes[15],
                ]);
                let length = u64::from_be_bytes([
                    bytes[16], bytes[17], bytes[18], bytes[19], bytes[20], bytes[21], bytes[22],
                    bytes[23],
                ]);

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
                let index = u64::from_be_bytes([
                    bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
                ]);
                let begin = u64::from_be_bytes([
                    bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14],
                    bytes[15],
                ]);
                let block = bytes[16..].to_vec();
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
    pub fn serialise(&self) -> Vec<u8> {
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
                let len = 1 + 8;
                let mut buf = u32::to_be_bytes(len).to_vec();
                buf.push(4);
                buf.append(&mut u64::to_be_bytes(*index).to_vec());
                buf
            }
            _ => todo!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn parse_keep_alive_message() {
        let len: u32 = 0;
        let buf = u32::to_le_bytes(len).to_vec();
        let expected_message = Message::KeepAlive;
        let mock_socket = tokio_test::io::Builder::new().read(&buf[..]).build();
        let res = Message::deserialise(mock_socket).await;
        assert!(res.is_ok_and(|message| message == expected_message))
    }

    #[tokio::test]
    async fn parse_choke_message() {
        let len: u32 = 1;
        let id = 0x00;
        let mut buf = u32::to_be_bytes(len).to_vec();
        buf.push(id);
        let expected_message = Message::Choke;
        let mock_socket = tokio_test::io::Builder::new().read(&buf[..]).build();
        let res = Message::deserialise(mock_socket).await;
        assert!(res.is_ok_and(|message| message == expected_message));
    }

    #[tokio::test]
    async fn parse_unchoke_message() {
        let len: u32 = 1;
        let id = 0x01;
        let mut buf = u32::to_be_bytes(len).to_vec();
        buf.push(id);
        let expected_message = Message::Unchoke;
        let mock_socket = tokio_test::io::Builder::new().read(&buf[..]).build();
        let res = Message::deserialise(mock_socket).await;
        assert!(res.is_ok_and(|message| message == expected_message));
    }

    #[tokio::test]
    async fn parse_interested_message() {
        let len: u32 = 1;
        let id = 0x02;
        let mut buf = u32::to_be_bytes(len).to_vec();
        buf.push(id);
        let expected_message = Message::Interested;
        let mock_socket = tokio_test::io::Builder::new().read(&buf[..]).build();
        let res = Message::deserialise(mock_socket).await;
        assert!(res.is_ok_and(|message| message == expected_message));
    }

    #[tokio::test]
    async fn parse_not_interested_message() {
        let len: u32 = 1;
        let id = 0x03;
        let mut buf = u32::to_be_bytes(len).to_vec();
        buf.push(id);
        let expected_message = Message::NotInterested;
        let mock_socket = tokio_test::io::Builder::new().read(&buf[..]).build();
        let res = Message::deserialise(mock_socket).await;
        assert!(res.is_ok_and(|message| message == expected_message));
    }

    #[tokio::test]
    async fn parse_have_message() {
        let len: u32 = 9;
        let id = 0x04;
        let index: u64 = 100;
        let mut buf = u32::to_be_bytes(len).to_vec();
        buf.push(id);
        buf.append(&mut u64::to_be_bytes(index).to_vec());
        let expected_message = Message::Have(index);
        let mock_socket = tokio_test::io::Builder::new().read(&buf[..]).build();
        let res = Message::deserialise(mock_socket).await;
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
        let mock_socket = tokio_test::io::Builder::new().read(&buf[..]).build();
        let res = Message::deserialise(mock_socket).await;
        assert!(res.is_ok_and(|message| message == expected_message));
    }

    #[tokio::test]
    async fn parse_request_message() {
        let len: u32 = 25;
        let id = 0x06;
        let index: u64 = 30;
        let begin: u64 = 100;
        let length: u64 = 200;
        let mut buf = u32::to_be_bytes(len).to_vec();
        buf.push(id);
        buf.append(&mut u64::to_be_bytes(index).to_vec());
        buf.append(&mut u64::to_be_bytes(begin).to_vec());
        buf.append(&mut u64::to_be_bytes(length).to_vec());
        let expected_message = Message::Request {
            index,
            begin,
            length,
        };
        let mock_socket = tokio_test::io::Builder::new().read(&buf[..]).build();
        let res = Message::deserialise(mock_socket).await;
        assert!(res.is_ok_and(|message| message == expected_message));
    }

    #[tokio::test]
    async fn parse_piece_message() {
        let id = 0x07;
        let index: u64 = 30;
        let begin: u64 = 100;
        let block = (0x00..0xFF).collect::<Vec<u8>>();
        let len: u32 = 1 + 8 + 8 + block.len() as u32;
        let mut buf = u32::to_be_bytes(len).to_vec();
        buf.push(id);
        buf.append(&mut u64::to_be_bytes(index).to_vec());
        buf.append(&mut u64::to_be_bytes(begin).to_vec());
        buf.append(&mut block.clone());
        let expected_message = Message::Piece {
            index,
            begin,
            block,
        };
        let mock_socket = tokio_test::io::Builder::new().read(&buf[..]).build();
        let res = Message::deserialise(mock_socket).await;
        assert!(res.is_ok_and(|message| message == expected_message));
    }

    #[tokio::test]
    async fn parse_cancel_message() {
        let len: u32 = 25;
        let id = 0x08;
        let index: u64 = 30;
        let begin: u64 = 100;
        let length: u64 = 200;
        let mut buf = u32::to_be_bytes(len).to_vec();
        buf.push(id);
        buf.append(&mut u64::to_be_bytes(index).to_vec());
        buf.append(&mut u64::to_be_bytes(begin).to_vec());
        buf.append(&mut u64::to_be_bytes(length).to_vec());
        let expected_message = Message::Cancel {
            index,
            begin,
            length,
        };
        let mock_socket = tokio_test::io::Builder::new().read(&buf[..]).build();
        let res = Message::deserialise(mock_socket).await;
        assert!(res.is_ok_and(|message| message == expected_message));
    }

    #[tokio::test]
    async fn return_error_if_single_byte_message_id_is_invalid() {
        let len: u32 = 1;
        let id = 0x04;
        let mut buf = u32::to_be_bytes(len).to_vec();
        buf.push(id);
        let mock_socket = tokio_test::io::Builder::new().read(&buf[..]).build();
        let res = Message::deserialise(mock_socket).await;
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
        let mock_socket = tokio_test::io::Builder::new().read(&buf[..]).build();
        let res = Message::deserialise(mock_socket).await;
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
        let len = 1 + 8;
        let id = 4;
        let mut expected_buf = u32::to_be_bytes(len).to_vec();
        expected_buf.push(id);
        expected_buf.append(&mut u64::to_be_bytes(index).to_vec());
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
