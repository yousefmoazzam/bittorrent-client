use crate::metainfo::Metainfo;
use crate::tracker::Peer;

/// Wrapper around [`Metainfo`]
pub struct Torrent {
    /// Metainfo file info
    pub metainfo: Metainfo,
    /// SHA1 hash of `info` dict
    pub info_hash: Vec<u8>,
    /// Peers associated with file
    pub peers: Vec<Peer>,
}

impl Torrent {
    /// Calculate SHA1 hash of `info` dict
    pub fn new(metainfo: Metainfo, peers: Vec<Peer>) -> Torrent {
        let bencoded_info = crate::serialise::serialise(metainfo.info.serialise());
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

    use crate::parse::BencodeType2;

    use super::*;

    #[test]
    fn correct_info_hash_field_on_torrent_instance() {
        let expected_info_hash =
            b"\x4a\xce\x56\xd9\xa0\x97\xed\xc1\x00\x57\xbb\x70\xf9\xd7\x98\xd5\x48\x44\xc8\xe9";
        let announce = b"hello";
        let mut metainfo_map = HashMap::new();
        metainfo_map.insert(
            b"announce".to_vec(),
            BencodeType2::ByteString(announce.to_vec()),
        );

        let name = b"hello";
        let length = 128;
        let piece_length = 64;
        let hello_sha1 =
            b"\xaa\xf4\xc6\x1d\xdc\xc5\xe8\xa2\xda\xbe\xde\x0f\x3b\x48\x2c\xd9\xae\xa9\x43\x4d";
        let goodbye_sha1 =
            b"\x3c\x8e\xc4\x87\x44\x88\xf6\x09\x0a\x15\x7b\x01\x4c\xe3\x39\x7c\xa8\xe0\x6d\x4f";
        let mut piece_hashes = hello_sha1.to_vec();
        piece_hashes.append(&mut goodbye_sha1.to_vec());
        let mut info_map = HashMap::new();
        info_map.insert(b"name".to_vec(), BencodeType2::ByteString(name.to_vec()));
        info_map.insert(b"length".to_vec(), BencodeType2::Integer(length));
        info_map.insert(
            b"piece length".to_vec(),
            BencodeType2::Integer(piece_length),
        );
        info_map.insert(b"pieces".to_vec(), BencodeType2::ByteString(piece_hashes));
        metainfo_map.insert(b"info".to_vec(), BencodeType2::Dict(info_map));
        let data = BencodeType2::Dict(metainfo_map);
        let metainfo = Metainfo::new(data).unwrap();

        let torrent = Torrent::new(metainfo, vec![]);
        assert_eq!(torrent.info_hash, expected_info_hash);
    }
}
