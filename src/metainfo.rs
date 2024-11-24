use crate::BencodeType;

/// Metainfo (`.torrent`) file
pub struct Metainfo;

const ANNOUNCE_KEY: &str = "announce";
const INFO_KEY: &str = "info";

impl Metainfo {
    pub fn new(data: BencodeType) -> Result<(), String> {
        match data {
            BencodeType::Dict(dict) => {
                for key in [ANNOUNCE_KEY, INFO_KEY] {
                    if !dict.contains_key(key) {
                        return Err(format!(
                            "Invalid input, metainfo dict missing the following key: {}",
                            key
                        ));
                    }
                }

                todo!()
            }
            _ => Err("Invalid input, metainfo file must be a dict".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn return_error_if_constructing_metainfo_from_incorrect_decoded_variant() {
        let incorrect_input = BencodeType::ByteString("hello".to_string());
        let res = Metainfo::new(incorrect_input);
        let expected_err_msg = "Invalid input, metainfo file must be a dict";
        assert_eq!(true, res.is_err_and(|msg| msg == expected_err_msg))
    }

    #[test]
    fn return_error_if_dict_missing_announce_key() {
        let info = BencodeType::Dict(HashMap::new());
        let mut map = HashMap::new();
        map.insert("info".to_string(), info);
        let incomplete_data = BencodeType::Dict(map);
        let expected_err_msg = "Invalid input, metainfo dict missing the following key: announce";
        let res = Metainfo::new(incomplete_data);
        assert_eq!(true, res.is_err_and(|msg| msg == expected_err_msg));
    }

    #[test]
    fn return_error_if_dict_missing_info_key() {
        let announce = BencodeType::ByteString("http://some.place.org:1234/announce".to_string());
        let mut map = HashMap::new();
        map.insert("announce".to_string(), announce);
        let incomplete_data = BencodeType::Dict(map);
        let expected_err_msg = "Invalid input, metainfo dict missing the following key: info";
        let res = Metainfo::new(incomplete_data);
        assert_eq!(true, res.is_err_and(|msg| msg == expected_err_msg));
    }
}
