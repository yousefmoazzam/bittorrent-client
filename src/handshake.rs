const PROTOCOL_ID_LEN: u8 = 0x13;
const PSTR: &str = "BitTorrent protocol";
const PEER_ID: &str = "-ABC123-abcd12345678";

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
}
