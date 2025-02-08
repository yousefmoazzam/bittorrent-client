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
}

pub fn parse(input: &[u8]) -> BencodeType2 {
    let mut parser = alt((parse_byte_string, parse_integer, parse_list));
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
    let element_parser = many1(parse_byte_string);
    let (leftover, middle) = delimited(tag("l"), element_parser, tag("e")).parse(input)?;
    Ok((leftover, BencodeType2::List(middle)))
}

#[cfg(test)]
mod tests {
    use super::parse;
    use crate::parse::BencodeType2;

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
}
