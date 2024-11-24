use crate::BencodeType;

/// Metainfo (`.torrent`) file
pub struct Metainfo;

impl Metainfo {
    pub fn new(data: BencodeType) -> Result<(), String> {
        match data {
            BencodeType::Dict(_) => todo!(),
            _ => Err("Invalid input, metainfo file must be a dict".to_string()),
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
}
