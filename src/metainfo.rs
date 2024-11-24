use std::collections::HashMap;

use crate::BencodeType;

/// Metainfo (`.torrent`) file
pub struct Metainfo;

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
    pub fn new(data: BencodeType) -> Result<(), String> {
        match data {
            BencodeType::Dict(dict) => {
                check_required_keys_exist(&dict)?;
                let _ = if let BencodeType::ByteString(val) = dict
                    .get(ANNOUNCE_KEY)
                    .expect("`announce` key has been confirmed to exist in hashmap")
                {
                    val
                } else {
                    return Err(
                        "Invalid input, the following key's value has an incorrect type: announce"
                            .to_string(),
                    );
                };

                let _ = if let BencodeType::Dict(_) = dict
                    .get(INFO_KEY)
                    .expect("`info` key has been confirmed to exist in hashmap")
                {
                    todo!()
                } else {
                    return Err(
                        "Invalid input, the following key's value has an incorrect type: info"
                            .to_string(),
                    );
                };
            }
            _ => Err("Invalid input, metainfo file must be a dict".to_string()),
        }
    }
}

const NAME_KEY: &str = "name";

/// Info dict within metainfo file
struct Info;

impl Info {
    fn new(data: BencodeType) -> Result<(), String> {
        match data {
            BencodeType::Dict(dict) => {
                for key in [NAME_KEY] {
                    if !dict.contains_key(key) {
                        return Err(format!(
                            "Invalid info dict, the following key is missing: {}",
                            key
                        ));
                    }
                }

                todo!()
            }
            _ => todo!(),
        }
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
        assert_eq!(true, res.is_err_and(|msg| msg == expected_err_msg));
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
        assert_eq!(true, res.is_err_and(|msg| msg == expected_err_msg));
    }

    #[test]
    fn return_error_if_info_dict_missing_name_key() {
        let data = BencodeType::Dict(HashMap::new());
        let expected_err_msg = "Invalid info dict, the following key is missing: name";
        let res = Info::new(data);
        assert_eq!(true, res.is_err_and(|msg| msg == expected_err_msg));
    }
}
