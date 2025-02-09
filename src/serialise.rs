use crate::parse::BencodeType2;

const COLON_ASCII: u8 = 58;

pub fn serialise(data: BencodeType2) -> Vec<u8> {
    match data {
        BencodeType2::Integer(int) => {
            let string = format!("i{}e", int);
            string.as_bytes().to_vec()
        }
        BencodeType2::ByteString(mut string) => {
            let mut out = string.len().to_string().as_bytes().to_owned();
            out.push(COLON_ASCII);
            out.append(&mut string);
            out
        }
        _ => todo!(),
    }
}

#[cfg(test)]
mod tests {
    use super::serialise;
    use crate::parse::BencodeType2;

    #[test]
    fn serialise_integer() {
        let integer = 42;
        let data = BencodeType2::Integer(integer);
        let expected_output = b"i42e".to_vec();
        let output = serialise(data);
        assert_eq!(output, expected_output);
    }

    #[test]
    fn serialise_bytestring() {
        let bytestring = b"hello";
        let data = BencodeType2::ByteString(bytestring.to_vec());
        let expected_output = b"5:hello".to_vec();
        let output = serialise(data);
        assert_eq!(output, expected_output);
    }
}
