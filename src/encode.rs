use crate::BencodeType;

/// Encode [`BencodeType`] to string
pub fn encode(data: BencodeType) -> String {
    match data {
        BencodeType::Integer(int) => format!("i{}e", int),
        _ => todo!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_integer() {
        let integer = 42;
        let data = BencodeType::Integer(integer);
        let expected_output = format!("i{}e", integer);
        let output = encode(data);
        assert_eq!(output, expected_output);
    }
}
