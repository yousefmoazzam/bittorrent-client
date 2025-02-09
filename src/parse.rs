use std::collections::HashMap;

use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::i64,
    multi::many1,
    sequence::{delimited, terminated},
    IResult, Parser,
};

#[derive(Debug, PartialEq)]
pub enum BencodeType2 {
    ByteString(Vec<u8>),
    Integer(i64),
    List(Vec<BencodeType2>),
    Dict(HashMap<Vec<u8>, BencodeType2>),
}

pub fn parse(input: &[u8]) -> BencodeType2 {
    let mut parser = alt((parse_byte_string, parse_integer, parse_list, parse_dict));
    match parser.parse(input) {
        Err(_) => todo!(),
        Ok((_, val)) => val,
    }
}

fn parse_byte_string(input: &[u8]) -> IResult<&[u8], BencodeType2> {
    let (leftover, len) = terminated(i64, tag(":")).parse(input)?;
    let val = leftover[..len as usize].to_vec();
    Ok((&leftover[len as usize..], BencodeType2::ByteString(val)))
}

fn parse_integer(input: &[u8]) -> IResult<&[u8], BencodeType2> {
    let (leftover, val) = delimited(tag("i"), i64, tag("e")).parse(input)?;
    Ok((leftover, BencodeType2::Integer(val)))
}

fn parse_list(input: &[u8]) -> IResult<&[u8], BencodeType2> {
    let element_parser = many1(alt((parse_byte_string, parse_integer, parse_list)));
    let (leftover, middle) = delimited(tag("l"), element_parser, tag("e")).parse(input)?;
    Ok((leftover, BencodeType2::List(middle)))
}

fn parse_dict(input: &[u8]) -> IResult<&[u8], BencodeType2> {
    let mut map = HashMap::new();
    let pair_parser = many1((
        parse_byte_string,
        alt((parse_byte_string, parse_integer, parse_list)),
    ));
    let (leftover, pairs) = delimited(tag("d"), pair_parser, tag("e")).parse(input)?;
    for (key, val) in pairs.into_iter() {
        match key {
            BencodeType2::ByteString(key) => {
                map.insert(key, val);
            }
            _ => todo!(),
        }
    }
    Ok((leftover, BencodeType2::Dict(map)))
}

#[cfg(test)]
mod tests {
    use super::parse;
    use crate::parse::BencodeType2;
    use std::collections::HashMap;

    #[test]
    fn parse_byte_string() {
        let data = b"5:hello5:world";
        let res = parse(&data[..]);
        match res {
            BencodeType2::ByteString(val) => assert_eq!(val, b"hello"),
            _ => panic!(),
        };
    }

    #[test]
    fn parse_integer() {
        let data = b"i42e5:hello";
        let res = parse(&data[..]);
        match res {
            BencodeType2::Integer(val) => assert_eq!(val, 42),
            _ => panic!(),
        }
    }

    #[test]
    fn parse_list_containing_single_bytestring() {
        let data = b"l5:helloe";
        let res = parse(&data[..]);
        match res {
            BencodeType2::List(val) => {
                assert_eq!(val, vec![BencodeType2::ByteString(b"hello".to_vec())])
            }
            _ => panic!(),
        }
    }

    #[test]
    fn parse_list_containing_multiple_bytestrings() {
        let data = b"l5:hello5:worlde";
        let res = parse(&data[..]);
        match res {
            BencodeType2::List(val) => {
                assert_eq!(
                    val,
                    vec![
                        BencodeType2::ByteString(b"hello".to_vec()),
                        BencodeType2::ByteString(b"world".to_vec())
                    ]
                )
            }
            _ => panic!(),
        }
    }

    #[test]
    fn parse_list_containing_bytestring_and_integer() {
        let data = b"li42e5:helloe";
        let res = parse(&data[..]);
        match res {
            BencodeType2::List(val) => {
                assert_eq!(
                    val,
                    vec![
                        BencodeType2::Integer(42),
                        BencodeType2::ByteString(b"hello".to_vec()),
                    ]
                )
            }
            _ => panic!(),
        }
    }

    #[test]
    fn parse_nested_list() {
        let data = b"li42el5:hello5:worldee";
        let res = parse(&data[..]);
        match res {
            BencodeType2::List(val) => {
                assert_eq!(
                    val,
                    vec![
                        BencodeType2::Integer(42),
                        BencodeType2::List(vec![
                            BencodeType2::ByteString(b"hello".to_vec()),
                            BencodeType2::ByteString(b"world".to_vec()),
                        ])
                    ]
                )
            }
            _ => panic!(),
        }
    }

    #[test]
    fn parse_dict_containing_single_key_bytestring_value() {
        let data = b"d5:hello5:worlde";
        let res = parse(&data[..]);
        let mut expected_map = HashMap::new();
        expected_map.insert(
            b"hello".to_vec(),
            BencodeType2::ByteString(b"world".to_vec()),
        );
        match res {
            BencodeType2::Dict(val) => assert_eq!(val, expected_map),
            _ => panic!(),
        }
    }

    #[test]
    fn parse_dict_containing_multiple_keys_bytestring_value() {
        let data = b"d5:hello5:world3:foo3:bare";
        let res = parse(&data[..]);
        let mut expected_map = HashMap::new();
        expected_map.insert(
            b"hello".to_vec(),
            BencodeType2::ByteString(b"world".to_vec()),
        );
        expected_map.insert(b"foo".to_vec(), BencodeType2::ByteString(b"bar".to_vec()));
        match res {
            BencodeType2::Dict(val) => assert_eq!(val, expected_map),
            _ => panic!(),
        }
    }

    #[test]
    fn parse_dict_containing_multiple_keys_bytestring_and_integer_values() {
        let data = b"d5:hello5:world3:fooi42ee";
        let res = parse(&data[..]);
        let mut expected_map = HashMap::new();
        expected_map.insert(
            b"hello".to_vec(),
            BencodeType2::ByteString(b"world".to_vec()),
        );
        expected_map.insert(b"foo".to_vec(), BencodeType2::Integer(42));
        match res {
            BencodeType2::Dict(val) => assert_eq!(val, expected_map),
            _ => panic!(),
        }
    }

    #[test]
    fn parse_dict_containing_single_key_list_value() {
        let data = b"d4:datal5:hello5:worldee";
        let res = parse(&data[..]);
        let mut expected_map = HashMap::new();
        expected_map.insert(
            b"data".to_vec(),
            BencodeType2::List(vec![
                BencodeType2::ByteString(b"hello".to_vec()),
                BencodeType2::ByteString(b"world".to_vec()),
            ]),
        );
        match res {
            BencodeType2::Dict(val) => assert_eq!(val, expected_map),
            _ => panic!(),
        }
    }
}
