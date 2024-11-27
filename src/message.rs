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

        match id {
            0x04 => {
                let bytes = &data[(ID_INDEX + 1) as usize..(ID_INDEX as u32 + len) as usize];
                let index = u64::from_be_bytes([
                    bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
                ]);
                Message::Have(index)
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
}
