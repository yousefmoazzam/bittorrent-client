use crate::BencodeType;

use regex::Regex;

/// Decode bencoded data
pub fn decode(data: &str) -> BencodeType {
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
}
