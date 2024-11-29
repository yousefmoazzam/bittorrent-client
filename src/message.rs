const ID_INDEX: u8 = 4;

/// Peer message types
#[derive(Debug, PartialEq)]
pub enum Message {
    Choke,
    Unchoke,
    Interested,
    NotInterested,
    /// Index of piece the downloader has completed and checked the hash of
    Have(u64),
    /// Describes which pieces (by index) the downloader has sent
    Bitfield(Vec<u8>),
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
}

impl Message {
    pub fn new(data: &[u8]) -> Message {
        let len = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        let id = data[ID_INDEX as usize];

        if len == 1 {
            return match id {
                0x00 => Message::Choke,
                0x01 => Message::Unchoke,
                0x02 => Message::Interested,
                0x03 => Message::NotInterested,
                _ => todo!(),
            };
        }

        let bytes = &data[(ID_INDEX + 1) as usize..(ID_INDEX as u32 + len) as usize];
        match id {
            0x04 => {
                let index = u64::from_be_bytes([
                    bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
                ]);
                Message::Have(index)
            }
            0x05 => Message::Bitfield(bytes.to_vec()),
            0x06 => {
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
                Message::Request {
                    index,
                    begin,
                    length,
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
                Message::Piece {
                    index,
                    begin,
                    block,
                }
            }
            _ => todo!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_choke_message() {
        let len: u32 = 1;
        let id = 0x00;
        let mut buf = u32::to_be_bytes(len).to_vec();
        buf.push(id);
        let expected_message = Message::Choke;
        let message = Message::new(&buf[..]);
        assert_eq!(message, expected_message);
    }

    #[test]
    fn parse_unchoke_message() {
        let len: u32 = 1;
        let id = 0x01;
        let mut buf = u32::to_be_bytes(len).to_vec();
        buf.push(id);
        let expected_message = Message::Unchoke;
        let message = Message::new(&buf[..]);
        assert_eq!(message, expected_message);
    }

    #[test]
    fn parse_interested_message() {
        let len: u32 = 1;
        let id = 0x02;
        let mut buf = u32::to_be_bytes(len).to_vec();
        buf.push(id);
        let expected_message = Message::Interested;
        let message = Message::new(&buf[..]);
        assert_eq!(message, expected_message);
    }

    #[test]
    fn parse_not_interested_message() {
        let len: u32 = 1;
        let id = 0x03;
        let mut buf = u32::to_be_bytes(len).to_vec();
        buf.push(id);
        let expected_message = Message::NotInterested;
        let message = Message::new(&buf[..]);
        assert_eq!(message, expected_message);
    }

    #[test]
    fn parse_have_message() {
        let len: u32 = 9;
        let id = 0x04;
        let index: u64 = 100;
        let mut buf = u32::to_be_bytes(len).to_vec();
        buf.push(id);
        buf.append(&mut u64::to_be_bytes(index).to_vec());
        let expected_message = Message::Have(index);
        let message = Message::new(&buf[..]);
        assert_eq!(message, expected_message);
    }

    #[test]
    fn parse_bitfield_message() {
        let len: u32 = 2;
        let id = 0x05;
        let bitfield = vec![0x10];
        let mut buf = u32::to_be_bytes(len).to_vec();
        buf.push(id);
        buf.append(&mut bitfield.clone());
        let expected_message = Message::Bitfield(bitfield);
        let message = Message::new(&buf[..]);
        assert_eq!(message, expected_message);
    }

    #[test]
    fn parse_request_message() {
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
        let message = Message::new(&buf[..]);
        assert_eq!(message, expected_message);
    }

    #[test]
    fn parse_piece_message() {
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
        let message = Message::new(&buf[..]);
        assert_eq!(message, expected_message);
    }
}
