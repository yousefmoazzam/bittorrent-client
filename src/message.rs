/// Peer message types
#[derive(Debug, PartialEq)]
pub enum Message {
    Choke,
}

impl Message {
    pub fn new(data: &[u8]) -> Message {
        let len = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        let id = data[4];

        if len == 1 {
            return match id {
                0x00 => Message::Choke,
                _ => todo!(),
            };
        }

        todo!()
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
}
