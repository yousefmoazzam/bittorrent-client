use crate::BencodeType;

use std::collections::HashMap;

use regex::Regex;

/// Decode bencoded data
pub fn decode(data: &str) -> BencodeType {
    let mut idx: usize = 0;
    decode_recurse(data, &mut idx)
}

/// Recursive helper to decode bencoded data
fn decode_recurse(data: &str, idx: &mut usize) -> BencodeType {
    let byte_str_len_regex = Regex::new(r"^\d+:").unwrap();
    let res = byte_str_len_regex.find(&data[*idx..]);
    if let Some(mat) = res {
        let str_len = mat.as_str()[..mat.len() - 1].parse::<usize>().unwrap();
        // Subtract 1 for the colon character
        let no_of_digits_in_str_len = mat.len() - 1;
        let (value, offset) = decode_byte_string(&data[*idx..], str_len, no_of_digits_in_str_len);
        *idx += offset;
        return value;
    }

    let type_char = data[*idx..].chars().next().unwrap();
    match type_char {
        'i' => {
            let (value, offset) = decode_integer(&data[*idx..]);
            *idx += offset;
            value
        }
        'l' => {
            *idx += 1;
            decode_list(data, idx)
        }
        'd' => {
            *idx += 1;
            decode_dict(data, idx)
        }
        _ => todo!(),
    }
}

/// Decode a bencoded dict
fn decode_dict(data: &str, idx: &mut usize) -> BencodeType {
    let mut map = HashMap::new();

    loop {
        let key = decode_recurse(data, idx);
        let value = decode_recurse(data, idx);

        if let BencodeType::ByteString(key_name) = key {
            map.insert(key_name, value);
        } else {
            todo!()
        }

        if data[*idx..].chars().next().unwrap() == 'e' {
            break;
        }
    }

    BencodeType::Dict(map)
}

/// Decode a bencoded list
fn decode_list(data: &str, idx: &mut usize) -> BencodeType {
    let mut elements = Vec::new();
    let mut next_char = data[*idx..].chars().next().unwrap();

    loop {
        match next_char {
            'e' => break,
            _ => {
                let element = decode_recurse(data, idx);
                elements.push(element);
                next_char = data[*idx..].chars().next().unwrap();
            }
        }
    }

    BencodeType::List(elements)
}

/// Decode a bencoded integer
fn decode_integer(data: &str) -> (BencodeType, usize) {
    let regex = Regex::new(r"^i-?\d+e").unwrap();
    let res = regex.find(data);
    let value = match res {
        Some(mat) => mat,
        None => todo!(),
    };
    let integer = value.as_str()[1..value.len() - 1].parse::<i64>().unwrap();
    (BencodeType::Integer(integer), value.len())
}

/// Decode a bencoded byte string
fn decode_byte_string(
    data: &str,
    str_len: usize,
    no_of_digits_in_len: usize,
) -> (BencodeType, usize) {
    let start = no_of_digits_in_len + 1;
    let stop = start + str_len;
    let string = &data[start..stop];
    (
        BencodeType::ByteString(string.to_string()),
        no_of_digits_in_len + 1 + string.len(),
    )
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

    #[test]
    fn decode_list_containing_strings_and_ints() {
        let integer = 42;
        let string = "hello";
        let bencoded_data = format!("li{}e{}:{}e", integer, string.len(), string);
        let expected_output = BencodeType::List(vec![
            BencodeType::Integer(integer),
            BencodeType::ByteString(string.to_string()),
        ]);
        let output = decode(&bencoded_data);
        assert_eq!(output, expected_output);
    }

    #[test]
    fn decode_dict_containing_strings_and_ints() {
        let key1 = "comment";
        let value1 = "Description of contents";
        let key2 = "creation date";
        let value2 = 1234567890;
        let bencoded_data = format!(
            "d{}:{}{}:{}{}:{}i{}ee",
            key1.len(),
            key1,
            value1.len(),
            value1,
            key2.len(),
            key2,
            value2,
        );
        let mut map = HashMap::new();
        map.insert(
            key1.to_string(),
            BencodeType::ByteString(value1.to_string()),
        );
        map.insert(key2.to_string(), BencodeType::Integer(value2));
        let expected_output = BencodeType::Dict(map);
        let output = decode(&bencoded_data);
        assert_eq!(output, expected_output);
    }
}
