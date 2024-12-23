use crate::metainfo::Metainfo;
use crate::tracker::Peer;

/// Wrapper around [`Metainfo`]
pub struct Torrent {
    /// Metainfo file info
    pub metainfo: Metainfo,
    // SHA1 hash of `info` dict
    pub info_hash: Vec<u8>,
    /// Peers associated with file
    pub peers: Vec<Peer>,
}

impl Torrent {
    /// Calculate SHA1 hash of `info` dict
    pub fn new(metainfo: Metainfo, peers: Vec<Peer>) -> Torrent {
        let bencoded_info = crate::encode::encode(metainfo.info.serialise());
        let hash = sha1_smol::Sha1::from(bencoded_info).digest().bytes();
        Torrent {
            metainfo,
            info_hash: hash.to_vec(),
            peers,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::BencodeType;

    use super::*;

    #[test]
    fn correct_info_hash_field_on_torrent_instance() {
        let expected_info_hash_str = "27364754102565e9ffd24ee504cab97fdea50b72";
        let mut expected_info_hash = Vec::new();
        let mut idx = 0;
        while idx < expected_info_hash_str.len() {
            let hex_val = &expected_info_hash_str[idx..idx + 2];
            expected_info_hash.push(u8::from_str_radix(hex_val, 16).unwrap());
            idx += 2;
        }

        let announce = "hello";
        let mut metainfo_map = HashMap::new();
        metainfo_map.insert(
            "announce".to_string(),
            BencodeType::ByteString(announce.to_string()),
        );

        let name = "hello";
        let length = 128;
        let piece_length = 64;
        let hello_sha1 = "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d";
        let goodbye_sha1 = "3c8ec4874488f6090a157b014ce3397ca8e06d4f";
        let mut info_map = HashMap::new();
        info_map.insert(
            "name".to_string(),
            BencodeType::ByteString(name.to_string()),
        );
        info_map.insert("length".to_string(), BencodeType::Integer(length));
        info_map.insert(
            "piece length".to_string(),
            BencodeType::Integer(piece_length),
        );
        info_map.insert(
            "pieces".to_string(),
            BencodeType::ByteString(format!("{}{}", hello_sha1, goodbye_sha1)),
        );
        metainfo_map.insert("info".to_string(), BencodeType::Dict(info_map));
        let data = BencodeType::Dict(metainfo_map);
        let metainfo = Metainfo::new(data).unwrap();

        let torrent = Torrent::new(metainfo, vec![]);
        assert_eq!(torrent.info_hash, expected_info_hash);
    }
}
