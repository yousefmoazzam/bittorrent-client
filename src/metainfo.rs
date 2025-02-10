use std::collections::HashMap;

use crate::parse::BencodeType2;

/// Metainfo (`.torrent`) file
#[derive(Clone)]
pub struct Metainfo {
    pub announce: String,
    pub info: Info,
}

const ANNOUNCE_KEY: &[u8] = b"announce";
const INFO_KEY: &[u8] = b"info";

/// Check the required keys for a valid metainfo dict exist in the given hashmap
fn check_required_keys_exist(dict: &HashMap<Vec<u8>, BencodeType2>) -> Result<(), String> {
    for key in [ANNOUNCE_KEY, INFO_KEY] {
        if !dict.contains_key(key) {
            return Err(format!(
                "Invalid input, metainfo dict missing the following key: {}",
                std::str::from_utf8(key).unwrap()
            ));
        }
    }
    Ok(())
}

impl Metainfo {
    pub fn new(data: BencodeType2) -> Result<Metainfo, String> {
        match data {
            BencodeType2::Dict(mut dict) => {
                check_required_keys_exist(&dict)?;
                let announce = if let BencodeType2::ByteString(val) = dict
                    .remove(ANNOUNCE_KEY)
                    .expect("`announce` key has been confirmed to exist in hashmap")
                {
                    val
                } else {
                    return Err(
                        "Invalid input, the following key's value has an incorrect type: announce"
                            .to_string(),
                    );
                };

                let info_decoded = dict
                    .remove(INFO_KEY)
                    .expect("`info` key has been confirmed to exist in hashmap");
                match info_decoded {
                    BencodeType2::ByteString(_)
                    | BencodeType2::Integer(_)
                    | BencodeType2::List(_) => Err(
                        "Invalid input, the following key's value has an incorrect type: info"
                            .to_string(),
                    ),
                    BencodeType2::Dict(_) => Ok(Metainfo {
                        announce: std::str::from_utf8(&announce).unwrap().to_string(),
                        info: Info::new(info_decoded)?,
                    }),
                }
            }
            _ => Err("Invalid input, metainfo file must be a dict".to_string()),
        }
    }
}

const NAME_KEY: &[u8] = b"name";
const LENGTH_KEY: &[u8] = b"length";
const PIECE_LENGTH_KEY: &[u8] = b"piece length";
const PIECES_KEY: &[u8] = b"pieces";
const SHA1_HASH_HEX_OUTPUT_SIZE: usize = 20;

/// Info dict within metainfo file
#[derive(Clone)]
pub struct Info {
    /// Name of the file
    pub name: String,
    /// Length of the file in bytes
    pub length: usize,
    /// Length of a piece of the file in bytes
    pub piece_length: usize,
    /// SHA1 hashes of pieces
    pub pieces: Vec<u8>,
}

impl Info {
    fn new(data: BencodeType2) -> Result<Info, String> {
        match data {
            BencodeType2::Dict(mut dict) => {
                for key in [NAME_KEY, LENGTH_KEY, PIECE_LENGTH_KEY, PIECES_KEY] {
                    if !dict.contains_key(key) {
                        return Err(format!(
                            "Invalid info dict, the following key is missing: {}",
                            std::str::from_utf8(key).unwrap()
                        ));
                    }
                }

                let name = if let BencodeType2::ByteString(val) = dict
                    .remove(NAME_KEY)
                    .expect("`name` key confirmed to exist in hashmap")
                {
                    val
                } else {
                    panic!()
                };

                let length = if let BencodeType2::Integer(val) = dict
                    .remove(LENGTH_KEY)
                    .expect("`name` key confirmed to exist in hashmap")
                {
                    usize::try_from(val).unwrap()
                } else {
                    panic!()
                };

                let piece_length = if let BencodeType2::Integer(val) = dict
                    .remove(PIECE_LENGTH_KEY)
                    .expect("`piece length` key confirmed to exist in hashmap")
                {
                    usize::try_from(val).unwrap()
                } else {
                    panic!()
                };

                let pieces = if let BencodeType2::ByteString(val) = dict
                    .remove(PIECES_KEY)
                    .expect("`pieces` key confirmed to exist in hashmap")
                {
                    val
                } else {
                    panic!()
                };

                Ok(Info {
                    name: std::str::from_utf8(&name).unwrap().to_string(),
                    length,
                    piece_length,
                    pieces,
                })
            }
            _ => todo!(),
        }
    }

    /// Get iterator over individual pieces in `pieces`
    pub fn pieces(&self) -> impl Iterator<Item = &[u8]> {
        let no_of_pieces = self.pieces.len() / SHA1_HASH_HEX_OUTPUT_SIZE;
        (0..no_of_pieces).map(|idx| {
            &self.pieces[idx * SHA1_HASH_HEX_OUTPUT_SIZE..(idx + 1) * SHA1_HASH_HEX_OUTPUT_SIZE]
        })
    }

    /// Serialise to [`BencodeType`]
    pub fn serialise(&self) -> BencodeType2 {
        let mut map = HashMap::new();
        map.insert(
            b"name".to_vec(),
            BencodeType2::ByteString(self.name.as_bytes().to_vec()),
        );
        map.insert(
            b"length".to_vec(),
            BencodeType2::Integer(i64::try_from(self.length).unwrap()),
        );
        map.insert(
            b"piece length".to_vec(),
            BencodeType2::Integer(i64::try_from(self.piece_length).unwrap()),
        );
        map.insert(
            b"pieces".to_vec(),
            BencodeType2::ByteString(self.pieces.clone()),
        );
        BencodeType2::Dict(map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn return_error_if_constructing_metainfo_from_incorrect_decoded_variant() {
        let incorrect_input = BencodeType2::ByteString(b"hello".to_vec());
        let res = Metainfo::new(incorrect_input);
        let expected_err_msg = "Invalid input, metainfo file must be a dict";
        assert!(res.is_err_and(|msg| msg == expected_err_msg))
    }

    #[test]
    fn return_error_if_dict_missing_announce_key() {
        let info = BencodeType2::Dict(HashMap::new());
        let mut map = HashMap::new();
        map.insert(b"info".to_vec(), info);
        let incomplete_data = BencodeType2::Dict(map);
        let expected_err_msg = "Invalid input, metainfo dict missing the following key: announce";
        let res = Metainfo::new(incomplete_data);
        assert!(res.is_err_and(|msg| msg == expected_err_msg));
    }

    #[test]
    fn return_error_if_dict_missing_info_key() {
        let announce = BencodeType2::ByteString(b"http://some.place.org:1234/announce".to_vec());
        let mut map = HashMap::new();
        map.insert(b"announce".to_vec(), announce);
        let incomplete_data = BencodeType2::Dict(map);
        let expected_err_msg = "Invalid input, metainfo dict missing the following key: info";
        let res = Metainfo::new(incomplete_data);
        assert!(res.is_err_and(|msg| msg == expected_err_msg));
    }

    #[test]
    fn return_error_if_announce_value_is_incorrect_decoded_variant() {
        let announce = BencodeType2::Integer(10);
        let info = BencodeType2::Dict(HashMap::new());
        let mut map = HashMap::new();
        map.insert(b"announce".to_vec(), announce);
        map.insert(b"info".to_vec(), info);
        let data = BencodeType2::Dict(map);
        let expected_err_msg =
            "Invalid input, the following key's value has an incorrect type: announce";
        let res = Metainfo::new(data);
        assert!(res.is_err_and(|msg| msg == expected_err_msg));
    }

    #[test]
    fn return_error_if_info_value_is_incorrect_decoded_variant() {
        let announce = BencodeType2::ByteString(b"hello".to_vec());
        let info = BencodeType2::List(vec![]);
        let mut map = HashMap::new();
        map.insert(b"announce".to_vec(), announce);
        map.insert(b"info".to_vec(), info);
        let data = BencodeType2::Dict(map);
        let expected_err_msg =
            "Invalid input, the following key's value has an incorrect type: info";
        let res = Metainfo::new(data);
        assert!(res.is_err_and(|msg| msg == expected_err_msg));
    }

    #[test]
    fn return_error_if_info_dict_missing_name_key() {
        let data = BencodeType2::Dict(HashMap::new());
        let expected_err_msg = "Invalid info dict, the following key is missing: name";
        let res = Info::new(data);
        assert!(res.is_err_and(|msg| msg == expected_err_msg));
    }

    #[test]
    fn return_error_if_info_dict_missing_length_key() {
        let mut map = HashMap::new();
        map.insert(
            b"name".to_vec(),
            BencodeType2::ByteString(b"hello".to_vec()),
        );
        let data = BencodeType2::Dict(map);
        let expected_err_msg = "Invalid info dict, the following key is missing: length";
        let res = Info::new(data);
        assert!(res.is_err_and(|msg| msg == expected_err_msg));
    }

    #[test]
    fn return_error_if_info_dict_missing_piece_length_key() {
        let mut map = HashMap::new();
        map.insert(
            b"name".to_vec(),
            BencodeType2::ByteString(b"hello".to_vec()),
        );
        map.insert(b"length".to_vec(), BencodeType2::Integer(64));
        let data = BencodeType2::Dict(map);
        let expected_err_msg = "Invalid info dict, the following key is missing: piece length";
        let res = Info::new(data);
        assert!(res.is_err_and(|msg| msg == expected_err_msg));
    }

    #[test]
    fn return_error_if_info_dict_missing_pieces_key() {
        let mut map = HashMap::new();
        map.insert(
            b"name".to_vec(),
            BencodeType2::ByteString(b"hello".to_vec()),
        );
        map.insert(b"length".to_vec(), BencodeType2::Integer(64));
        map.insert(b"piece length".to_vec(), BencodeType2::Integer(256));
        let data = BencodeType2::Dict(map);
        let expected_err_msg = "Invalid info dict, the following key is missing: pieces";
        let res = Info::new(data);
        assert!(res.is_err_and(|msg| msg == expected_err_msg));
    }

    #[test]
    fn get_expected_info_struct_from_valid_info_dict() {
        let name = b"hello";
        let length = 128;
        let piece_length = 64;
        let hello_sha1 =
            b"\xaa\xf4\xc6\x1d\xdc\xc5\xe8\xa2\xda\xbe\xde\x0f\x3b\x48\x2c\xd9\xae\xa9\x43\x4d";
        let goodbye_sha1 =
            b"\x3c\x8e\xc4\x87\x44\x88\xf6\x09\x0a\x15\x7b\x01\x4c\xe3\x39\x7c\xa8\xe0\x6d\x4f";
        let mut piece_hashes = hello_sha1.to_vec();
        piece_hashes.append(&mut goodbye_sha1.to_vec());
        let mut map = HashMap::new();
        map.insert(b"name".to_vec(), BencodeType2::ByteString(name.to_vec()));
        map.insert(b"length".to_vec(), BencodeType2::Integer(length));
        map.insert(
            b"piece length".to_vec(),
            BencodeType2::Integer(piece_length),
        );
        map.insert(
            b"pieces".to_vec(),
            BencodeType2::ByteString(piece_hashes.clone()),
        );
        let data = BencodeType2::Dict(map);
        let info = Info::new(data).unwrap();
        assert_eq!(info.name, std::str::from_utf8(name).unwrap());
        assert_eq!(info.length, usize::try_from(length).unwrap());
        assert_eq!(info.piece_length, usize::try_from(piece_length).unwrap());
        assert_eq!(info.pieces, piece_hashes);
    }

    #[test]
    fn get_pieces_iterator_from_info_struct() {
        let name = b"hello";
        let length = 128;
        let piece_length = 64;
        let hello_sha1 =
            b"\xaa\xf4\xc6\x1d\xdc\xc5\xe8\xa2\xda\xbe\xde\x0f\x3b\x48\x2c\xd9\xae\xa9\x43\x4d";
        let goodbye_sha1 =
            b"\x3c\x8e\xc4\x87\x44\x88\xf6\x09\x0a\x15\x7b\x01\x4c\xe3\x39\x7c\xa8\xe0\x6d\x4f";
        let mut piece_hashes = hello_sha1.to_vec();
        piece_hashes.append(&mut goodbye_sha1.to_vec());
        let mut map = HashMap::new();
        map.insert(b"name".to_vec(), BencodeType2::ByteString(name.to_vec()));
        map.insert(b"length".to_vec(), BencodeType2::Integer(length));
        map.insert(
            b"piece length".to_vec(),
            BencodeType2::Integer(piece_length),
        );
        map.insert(b"pieces".to_vec(), BencodeType2::ByteString(piece_hashes));
        let data = BencodeType2::Dict(map);
        let info = Info::new(data).unwrap();
        let pieces = info.pieces().collect::<Vec<&[u8]>>();
        assert_eq!(pieces.len(), 2);
        for (piece, expected_piece) in std::iter::zip([hello_sha1, goodbye_sha1], pieces) {
            assert_eq!(piece, expected_piece);
        }
    }

    #[test]
    fn info_serialisation_produces_correct_bencode_type_dict_variant() {
        let name = b"hello";
        let length = 128;
        let piece_length = 64;
        let hello_sha1 =
            b"\xaa\xf4\xc6\x1d\xdc\xc5\xe8\xa2\xda\xbe\xde\x0f\x3b\x48\x2c\xd9\xae\xa9\x43\x4d";
        let goodbye_sha1 =
            b"\x3c\x8e\xc4\x87\x44\x88\xf6\x09\x0a\x15\x7b\x01\x4c\xe3\x39\x7c\xa8\xe0\x6d\x4f";
        let mut piece_hashes = hello_sha1.to_vec();
        piece_hashes.append(&mut goodbye_sha1.to_vec());
        let info = Info {
            name: std::str::from_utf8(name).unwrap().to_string(),
            length,
            piece_length,
            pieces: piece_hashes.clone(),
        };
        let expected_pairs = vec![
            (
                b"length".to_vec(),
                BencodeType2::Integer(i64::try_from(length).unwrap()),
            ),
            (b"name".to_vec(), BencodeType2::ByteString(name.to_vec())),
            (
                b"piece length".to_vec(),
                BencodeType2::Integer(i64::try_from(piece_length).unwrap()),
            ),
            (b"pieces".to_vec(), BencodeType2::ByteString(piece_hashes)),
        ];
        let sorted_pairs = match info.serialise() {
            BencodeType2::Dict(map) => {
                let mut pairs = map.into_iter().collect::<Vec<_>>();
                pairs.sort_by_key(|pair| pair.0.clone());
                pairs
            }
            _ => panic!("Expected `Dict` variant"),
        };
        assert_eq!(expected_pairs, sorted_pairs);
    }

    #[test]
    fn get_expected_metainfo_struct_from_valid_dict() {
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
        assert_eq!(metainfo.announce, std::str::from_utf8(announce).unwrap());
    }
}
