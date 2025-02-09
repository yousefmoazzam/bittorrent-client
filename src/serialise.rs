use crate::parse::BencodeType2;

pub fn serialise(data: BencodeType2) -> Vec<u8> {
    match data {
        BencodeType2::Integer(int) => {
            let string = format!("i{}e", int);
            string.as_bytes().to_vec()
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
}
