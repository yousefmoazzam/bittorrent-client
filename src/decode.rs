use crate::BencodeType;

use regex::Regex;

/// Decode bencoded data
pub fn decode(data: &str) -> BencodeType {
    let byte_str_len_regex = Regex::new(r"\d+:").unwrap();
    let res = byte_str_len_regex.find(data);
    if let Some(mat) = res {
        let str_len = mat.as_str()[..mat.len() - 1].parse::<usize>().unwrap();
        // Subtract 1 for the colon character
        let no_of_digits_in_str_len = mat.len() - 1;
        return decode_byte_string(data, str_len, no_of_digits_in_str_len);
    }

    let type_char = data.chars().next().unwrap();
    match type_char {
        'i' => decode_integer(&data[..]),
        _ => todo!(),
    }
}

/// Decode a bencoded integer
fn decode_integer(data: &str) -> BencodeType {
    let regex = Regex::new(r"i-?\d+e").unwrap();
    let res = regex.find(data);
    let value = match res {
        Some(mat) => mat,
        None => todo!(),
    };
    let integer = value.as_str()[1..value.len() - 1].parse::<i64>().unwrap();
    BencodeType::Integer(integer)
}

/// Decode a bencoded byte string
fn decode_byte_string(data: &str, str_len: usize, no_of_digits_in_len: usize) -> BencodeType {
    let start = no_of_digits_in_len + 1;
    let stop = start + str_len;
    let string = &data[start..stop];
    BencodeType::ByteString(string.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_positive_integer() {
        let integer = 42;
        let bencoded_data = format!("i{}e", integer);
        let expected_output = BencodeType::Integer(integer);
        let output = decode(&bencoded_data[..]);
        assert_eq!(output, expected_output);
    }

    #[test]
    fn decode_negative_integer() {
        let integer = -42;
        let bencoded_data = format!("i{}e", integer);
        let expected_output = BencodeType::Integer(integer);
        let output = decode(&bencoded_data[..]);
        assert_eq!(output, expected_output);
    }

    #[test]
    fn decode_byte_string() {
        let string = "hello";
        let bencoded_data = format!("{}:{}", string.len(), string);
        let expected_output = BencodeType::ByteString(string.to_string());
        let output = decode(&bencoded_data);
        assert_eq!(output, expected_output);
    }
}
