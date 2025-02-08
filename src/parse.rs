use nom::{
    bytes::complete::tag,
    character::complete::i64,
    sequence::{delimited, terminated},
    IResult, Parser,
};

pub enum BencodeType2 {
    ByteString(Vec<u8>),
    Integer(i64),
}

pub fn parse(input: &[u8]) -> BencodeType2 {
    match parse_byte_string(input) {
        Err(_) => match parse_integer(input) {
            Err(_) => todo!(),
            Ok((_, val)) => val,
        },
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
}
