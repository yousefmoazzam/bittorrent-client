use std::collections::HashMap;

use crate::BencodeType;

/// Metainfo (`.torrent`) file
pub struct Metainfo {
    announce: String,
    pub info: Info,
}

const ANNOUNCE_KEY: &str = "announce";
const INFO_KEY: &str = "info";

/// Check the required keys for a valid metainfo dict exist in the given hashmap
fn check_required_keys_exist(dict: &HashMap<String, BencodeType>) -> Result<(), String> {
    for key in [ANNOUNCE_KEY, INFO_KEY] {
        if !dict.contains_key(key) {
            return Err(format!(
                "Invalid input, metainfo dict missing the following key: {}",
                key
            ));
        }
    }
    Ok(())
}

impl Metainfo {
    pub fn new(data: BencodeType) -> Result<Metainfo, String> {
        match data {
            BencodeType::Dict(mut dict) => {
                check_required_keys_exist(&dict)?;
                let announce = if let BencodeType::ByteString(val) = dict
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
                    BencodeType::ByteString(_) | BencodeType::Integer(_) | BencodeType::List(_) => {
                        Err(
                            "Invalid input, the following key's value has an incorrect type: info"
                                .to_string(),
                        )
                    }
                    BencodeType::Dict(_) => Ok(Metainfo {
                        announce,
                        info: Info::new(info_decoded)?,
                    }),
                }
            }
            _ => Err("Invalid input, metainfo file must be a dict".to_string()),
        }
    }
}

const NAME_KEY: &str = "name";
const LENGTH_KEY: &str = "length";
const PIECE_LENGTH_KEY: &str = "piece length";
const PIECES_KEY: &str = "pieces";
const SHA1_HASH_HEX_OUTPUT_SIZE: usize = 40;

/// Info dict within metainfo file
pub struct Info {
    /// Name of the file
    name: String,
    /// Length of the file in bytes
    length: usize,
    /// Length of a piece of the file in bytes
    piece_length: usize,
    /// String containing full pieces bencoded byte string
    pieces: String,
}

impl Info {
    fn new(data: BencodeType) -> Result<Info, String> {
        match data {
            BencodeType::Dict(mut dict) => {
                for key in [NAME_KEY, LENGTH_KEY, PIECE_LENGTH_KEY, PIECES_KEY] {
                    if !dict.contains_key(key) {
                        return Err(format!(
                            "Invalid info dict, the following key is missing: {}",
                            key
                        ));
                    }
                }

                let name = if let BencodeType::ByteString(val) = dict
                    .remove(NAME_KEY)
                    .expect("`name` key confirmed to exist in hashmap")
                {
                    val
                } else {
                    panic!()
                };

                let length = if let BencodeType::Integer(val) = dict
                    .remove(LENGTH_KEY)
                    .expect("`name` key confirmed to exist in hashmap")
                {
                    usize::try_from(val).unwrap()
                } else {
                    panic!()
                };

                let piece_length = if let BencodeType::Integer(val) = dict
                    .remove(PIECE_LENGTH_KEY)
                    .expect("`piece length` key confirmed to exist in hashmap")
                {
                    usize::try_from(val).unwrap()
                } else {
                    panic!()
                };

                let pieces = if let BencodeType::ByteString(val) = dict
                    .remove(PIECES_KEY)
                    .expect("`pieces` key confirmed to exist in hashmap")
                {
                    val
                } else {
                    panic!()
                };

                Ok(Info {
                    name,
                    length,
                    piece_length,
                    pieces,
                })
            }
            _ => todo!(),
        }
    }

    /// Get iterator over individual pieces in `pieces`
    pub fn pieces(&self) -> impl Iterator<Item = &str> {
        let no_of_pieces = self.pieces.len() / SHA1_HASH_HEX_OUTPUT_SIZE;
        (0..no_of_pieces).map(|idx| {
            &self.pieces[idx * SHA1_HASH_HEX_OUTPUT_SIZE..(idx + 1) * SHA1_HASH_HEX_OUTPUT_SIZE]
        })
    }

    /// Serialise to [`BencodeType`]
    pub fn serialise(&self) -> BencodeType {
        let mut map = HashMap::new();
        map.insert(
            "name".to_string(),
            BencodeType::ByteString(self.name.clone()),
        );
        map.insert(
            "length".to_string(),
            BencodeType::Integer(i64::try_from(self.length).unwrap()),
        );
        map.insert(
            "piece length".to_string(),
            BencodeType::Integer(i64::try_from(self.piece_length).unwrap()),
        );
        map.insert(
            "pieces".to_string(),
            BencodeType::ByteString(self.pieces.clone()),
        );
        BencodeType::Dict(map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn return_error_if_constructing_metainfo_from_incorrect_decoded_variant() {
        let incorrect_input = BencodeType::ByteString("hello".to_string());
        let res = Metainfo::new(incorrect_input);
        let expected_err_msg = "Invalid input, metainfo file must be a dict";
        assert!(res.is_err_and(|msg| msg == expected_err_msg))
    }

    #[test]
    fn return_error_if_dict_missing_announce_key() {
        let info = BencodeType::Dict(HashMap::new());
        let mut map = HashMap::new();
        map.insert("info".to_string(), info);
        let incomplete_data = BencodeType::Dict(map);
        let expected_err_msg = "Invalid input, metainfo dict missing the following key: announce";
        let res = Metainfo::new(incomplete_data);
        assert!(res.is_err_and(|msg| msg == expected_err_msg));
    }

    #[test]
    fn return_error_if_dict_missing_info_key() {
        let announce = BencodeType::ByteString("http://some.place.org:1234/announce".to_string());
        let mut map = HashMap::new();
        map.insert("announce".to_string(), announce);
        let incomplete_data = BencodeType::Dict(map);
        let expected_err_msg = "Invalid input, metainfo dict missing the following key: info";
        let res = Metainfo::new(incomplete_data);
        assert!(res.is_err_and(|msg| msg == expected_err_msg));
    }

    #[test]
    fn return_error_if_announce_value_is_incorrect_decoded_variant() {
        let announce = BencodeType::Integer(10);
        let info = BencodeType::Dict(HashMap::new());
        let mut map = HashMap::new();
        map.insert("announce".to_string(), announce);
        map.insert("info".to_string(), info);
        let data = BencodeType::Dict(map);
        let expected_err_msg =
            "Invalid input, the following key's value has an incorrect type: announce";
        let res = Metainfo::new(data);
        assert!(res.is_err_and(|msg| msg == expected_err_msg));
    }

    #[test]
    fn return_error_if_info_value_is_incorrect_decoded_variant() {
        let announce = BencodeType::ByteString("hello".to_string());
        let info = BencodeType::List(vec![]);
        let mut map = HashMap::new();
        map.insert("announce".to_string(), announce);
        map.insert("info".to_string(), info);
        let data = BencodeType::Dict(map);
        let expected_err_msg =
            "Invalid input, the following key's value has an incorrect type: info";
        let res = Metainfo::new(data);
        assert!(res.is_err_and(|msg| msg == expected_err_msg));
    }

    #[test]
    fn return_error_if_info_dict_missing_name_key() {
        let data = BencodeType::Dict(HashMap::new());
        let expected_err_msg = "Invalid info dict, the following key is missing: name";
        let res = Info::new(data);
        assert!(res.is_err_and(|msg| msg == expected_err_msg));
    }

    #[test]
    fn return_error_if_info_dict_missing_length_key() {
        let mut map = HashMap::new();
        map.insert(
            "name".to_string(),
            BencodeType::ByteString("hello".to_string()),
        );
        let data = BencodeType::Dict(map);
        let expected_err_msg = "Invalid info dict, the following key is missing: length";
        let res = Info::new(data);
        assert!(res.is_err_and(|msg| msg == expected_err_msg));
    }

    #[test]
    fn return_error_if_info_dict_missing_piece_length_key() {
        let mut map = HashMap::new();
        map.insert(
            "name".to_string(),
            BencodeType::ByteString("hello".to_string()),
        );
        map.insert("length".to_string(), BencodeType::Integer(64));
        let data = BencodeType::Dict(map);
        let expected_err_msg = "Invalid info dict, the following key is missing: piece length";
        let res = Info::new(data);
        assert!(res.is_err_and(|msg| msg == expected_err_msg));
    }

    #[test]
    fn return_error_if_info_dict_missing_pieces_key() {
        let mut map = HashMap::new();
        map.insert(
            "name".to_string(),
            BencodeType::ByteString("hello".to_string()),
        );
        map.insert("length".to_string(), BencodeType::Integer(64));
        map.insert("piece length".to_string(), BencodeType::Integer(256));
        let data = BencodeType::Dict(map);
        let expected_err_msg = "Invalid info dict, the following key is missing: pieces";
        let res = Info::new(data);
        assert!(res.is_err_and(|msg| msg == expected_err_msg));
    }

    #[test]
    fn get_expected_info_struct_from_valid_info_dict() {
        let name = "hello";
        let length = 128;
        let piece_length = 64;
        let hello_sha1 = "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d";
        let goodbye_sha1 = "3c8ec4874488f6090a157b014ce3397ca8e06d4f";
        let mut map = HashMap::new();
        map.insert(
            "name".to_string(),
            BencodeType::ByteString(name.to_string()),
        );
        map.insert("length".to_string(), BencodeType::Integer(length));
        map.insert(
            "piece length".to_string(),
            BencodeType::Integer(piece_length),
        );
        map.insert(
            "pieces".to_string(),
            BencodeType::ByteString(format!("{}{}", hello_sha1, goodbye_sha1)),
        );
        let data = BencodeType::Dict(map);
        let info = Info::new(data).unwrap();
        assert_eq!(info.name, name.to_string());
        assert_eq!(info.length, usize::try_from(length).unwrap());
        assert_eq!(info.piece_length, usize::try_from(piece_length).unwrap());
        assert_eq!(info.pieces, format!("{}{}", hello_sha1, goodbye_sha1));
    }

    #[test]
    fn get_pieces_iterator_from_info_struct() {
        let name = "hello";
        let length = 128;
        let piece_length = 64;
        let hello_sha1 = "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d";
        let goodbye_sha1 = "3c8ec4874488f6090a157b014ce3397ca8e06d4f";
        let mut map = HashMap::new();
        map.insert(
            "name".to_string(),
            BencodeType::ByteString(name.to_string()),
        );
        map.insert("length".to_string(), BencodeType::Integer(length));
        map.insert(
            "piece length".to_string(),
            BencodeType::Integer(piece_length),
        );
        map.insert(
            "pieces".to_string(),
            BencodeType::ByteString(format!("{}{}", hello_sha1, goodbye_sha1)),
        );
        let data = BencodeType::Dict(map);
        let info = Info::new(data).unwrap();
        let pieces = info.pieces().collect::<Vec<&str>>();
        assert_eq!(pieces.len(), 2);
        for (piece, expected_piece) in std::iter::zip([hello_sha1, goodbye_sha1], pieces) {
            assert_eq!(piece, expected_piece);
        }
    }

    #[test]
    fn info_serialisation_produces_correct_bencode_type_dict_variant() {
        let name = "hello";
        let length = 128;
        let piece_length = 64;
        let hello_sha1 = "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d";
        let goodbye_sha1 = "3c8ec4874488f6090a157b014ce3397ca8e06d4f";
        let pieces = format!("{}{}", hello_sha1, goodbye_sha1);
        let info = Info {
            name: name.to_string(),
            length,
            piece_length,
            pieces,
        };
        let expected_pairs = vec![
            (
                "length".to_string(),
                BencodeType::Integer(i64::try_from(length).unwrap()),
            ),
            (
                "name".to_string(),
                BencodeType::ByteString(name.to_string()),
            ),
            (
                "piece length".to_string(),
                BencodeType::Integer(i64::try_from(piece_length).unwrap()),
            ),
            (
                "pieces".to_string(),
                BencodeType::ByteString(format!("{}{}", hello_sha1, goodbye_sha1)),
            ),
        ];
        let sorted_pairs = match info.serialise() {
            BencodeType::Dict(map) => {
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
        assert_eq!(metainfo.announce, announce);
    }
}
