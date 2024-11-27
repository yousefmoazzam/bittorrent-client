const PROTOCOL_ID_LEN: u8 = 0x13;
const PSTR: &str = "BitTorrent protocol";
const PEER_ID: &str = "-ABC123-abcd12345678";
const NO_OF_RESERVED_BYTES: u8 = 8;
const INFO_HASH_LEN_BYTES: u8 = 20;
const PEER_ID_LEN_BYTES: u8 = 20;

/// BitTorrent handshake
pub struct Handshake {
    /// Protocol identifier
    pstr: String,
    /// SHA1 hash of bencoded `info` dict of file
    info_hash: Vec<u8>,
    /// Identifier of peer
    peer_id: Vec<u8>,
}

impl Handshake {
    pub fn new(pstr: String, info_hash: Vec<u8>, peer_id: Vec<u8>) -> Handshake {
        Handshake {
            pstr,
            info_hash,
            peer_id,
        }
    }

    /// Deserialise data into handshake
    pub fn deserialise(data: &[u8]) -> Handshake {
        let protocol_identifier_len = data[0];
        let protocol_identifier = &data[1..(1 + protocol_identifier_len) as usize];
        let _reserved_bytes = &data[(1 + protocol_identifier_len) as usize
            ..(1 + protocol_identifier_len + NO_OF_RESERVED_BYTES) as usize];
        let info_hash = &data[(1 + protocol_identifier_len + NO_OF_RESERVED_BYTES) as usize
            ..(1 + protocol_identifier_len + NO_OF_RESERVED_BYTES + INFO_HASH_LEN_BYTES) as usize];
        let peer_id =
            &data[(1 + protocol_identifier_len + NO_OF_RESERVED_BYTES + INFO_HASH_LEN_BYTES)
                as usize
                ..(1 + protocol_identifier_len
                    + NO_OF_RESERVED_BYTES
                    + INFO_HASH_LEN_BYTES
                    + PEER_ID_LEN_BYTES) as usize];
        Handshake {
            pstr: std::str::from_utf8(protocol_identifier)
                .unwrap()
                .to_string(),
            info_hash: info_hash.to_vec(),
            peer_id: peer_id.to_vec(),
        }
    }

    /// Serialise handshake data
    pub fn serialise(&self) -> Vec<u8> {
        let mut output = vec![PROTOCOL_ID_LEN];
        output.append(&mut self.pstr.as_bytes().to_vec());
        output.append(&mut vec![0x00; 8]);
        output.append(&mut self.info_hash.clone());
        output.append(&mut self.peer_id.clone());
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialised_handshake_is_correct() {
        let info_hash = (0x00..0x14).collect::<Vec<_>>();
        let mut expected = vec![PROTOCOL_ID_LEN];
        expected.append(&mut PSTR.as_bytes().to_vec());
        expected.append(&mut vec![0x00; 8]);
        expected.append(&mut info_hash.clone());
        expected.append(&mut PEER_ID.as_bytes().to_vec());
        let handshake = Handshake::new(
            PSTR.to_string(),
            info_hash,
            PEER_ID.to_string().as_bytes().to_vec(),
        );
        assert_eq!(handshake.serialise(), expected);
    }

    #[test]
    fn deserialised_handshake_is_correct() {
        let info_hash = (0x00..0x14).collect::<Vec<_>>();
        let mut buffer = vec![PROTOCOL_ID_LEN];
        buffer.append(&mut PSTR.as_bytes().to_vec());
        buffer.append(&mut vec![0x00; 8]);
        buffer.append(&mut info_hash.clone());
        buffer.append(&mut PEER_ID.as_bytes().to_vec());
        let handshake = Handshake::deserialise(&buffer[..]);
        assert_eq!(PSTR.to_string(), handshake.pstr);
        assert_eq!(PEER_ID.to_string().as_bytes().to_vec(), handshake.peer_id);
        assert_eq!(info_hash, handshake.info_hash);
    }
}
