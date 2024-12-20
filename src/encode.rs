use crate::BencodeType;

/// Encode [`BencodeType`] to string
pub fn encode(data: BencodeType) -> String {
    match data {
        BencodeType::Integer(int) => format!("i{}e", int),
        BencodeType::ByteString(string) => format!("{}:{}", string.len(), string),
        BencodeType::List(items) => {
            let inner = items.into_iter().map(encode).collect::<Vec<_>>().join("");
            format!("l{}e", inner)
        }
        BencodeType::Dict(dict) => {
            let mut pairs = dict.into_iter().collect::<Vec<(String, BencodeType)>>();
            pairs.sort_by_key(|pair| pair.0.clone());
            let inner = pairs
                .into_iter()
                .map(|(key, value)| {
                    let mut encoded_key = encode(BencodeType::ByteString(key));
                    let encoded_value = encode(value);
                    encoded_key.push_str(&encoded_value);
                    encoded_key
                })
                .collect::<Vec<_>>()
                .join("");
            format!("d{}e", inner)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn encode_integer() {
        let integer = 42;
        let data = BencodeType::Integer(integer);
        let expected_output = format!("i{}e", integer);
        let output = encode(data);
        assert_eq!(output, expected_output);
    }

    #[test]
    fn encode_byte_string() {
        let string = "hello";
        let data = BencodeType::ByteString(string.to_string());
        let expected_output = format!("{}:{}", string.len(), string);
        let output = encode(data);
        assert_eq!(output, expected_output);
    }

    #[test]
    fn encode_list() {
        let integer = 42;
        let string = "hello";
        let data = BencodeType::List(vec![
            BencodeType::Integer(integer),
            BencodeType::ByteString(string.to_string()),
        ]);
        let expected_output = format!("li{}e{}:{}e", integer, string.len(), string);
        let output = encode(data);
        assert_eq!(output, expected_output);
    }

    #[test]
    fn encode_dict() {
        let key1 = "comment";
        let value1 = "Description of contents";
        let key2 = "creation date";
        let value2 = 1234567890;
        let mut map = HashMap::new();
        map.insert(
            key1.to_string(),
            BencodeType::ByteString(value1.to_string()),
        );
        map.insert(key2.to_string(), BencodeType::Integer(value2));
        let data = BencodeType::Dict(map);
        let expected_output = format!(
            "d{}:{}{}:{}{}:{}i{}ee",
            key1.len(),
            key1,
            value1.len(),
            value1,
            key2.len(),
            key2,
            value2,
        );
        let output = encode(data);
        assert_eq!(output, expected_output);
    }
}
