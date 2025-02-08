use nom::{bytes::complete::tag, character::complete::i64, IResult, Parser};

pub enum BencodeType2 {
    ByteString(Vec<u8>),
}

pub fn parse(input: &[u8]) -> BencodeType2 {
    match parse_byte_string(input) {
        Err(_) => todo!(),
        Ok((leftover, (len, _))) => BencodeType2::ByteString(leftover[..len as usize].to_vec()),
    }
}

fn parse_byte_string(input: &[u8]) -> IResult<&[u8], (i64, &[u8])> {
    (i64, tag(":")).parse(input)
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
        };
    }
}
