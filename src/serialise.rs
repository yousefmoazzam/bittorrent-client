use crate::parse::BencodeType2;

const COLON_ASCII: u8 = 58;
const L_ASCII: u8 = 108;
const E_ASCII: u8 = 101;
const D_ASCII: u8 = 100;

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
        BencodeType2::List(list) => {
            let mut elems = vec![L_ASCII];
            for item in list.into_iter() {
                elems.append(&mut serialise(item));
            }
            elems.push(E_ASCII);
            elems
        }
        BencodeType2::Dict(map) => {
            let mut pairs = map.into_iter().collect::<Vec<(Vec<u8>, BencodeType2)>>();
            pairs.sort_by_key(|pair| pair.0.clone());
            let mut out = vec![D_ASCII];
            for (key, val) in pairs.into_iter() {
                let bencoded_key = BencodeType2::ByteString(key);
                out.append(&mut serialise(bencoded_key));
                out.append(&mut serialise(val));
            }
            out.push(E_ASCII);
            out
        }
    }
}

#[cfg(test)]
mod tests {
    use super::serialise;
    use crate::parse::BencodeType2;
    use std::collections::HashMap;

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

    #[test]
    fn serialise_list() {
        let integer = 42;
        let bytestring = b"hello";
        let data = BencodeType2::List(vec![
            BencodeType2::ByteString(bytestring.to_vec()),
            BencodeType2::Integer(integer),
        ]);
        let expected_output = b"l5:helloi42ee";
        let output = serialise(data);
        assert_eq!(output, expected_output);
    }

    #[test]
    fn serialise_dict() {
        let key1 = b"comment";
        let value1 = b"Description of contents";
        let key2 = b"creation date";
        let value2 = 1234567890;
        let mut map = HashMap::new();
        map.insert(key1.to_vec(), BencodeType2::ByteString(value1.to_vec()));
        map.insert(key2.to_vec(), BencodeType2::Integer(value2));
        let data = BencodeType2::Dict(map);
        let expected_output = b"d7:comment23:Description of contents13:creation datei1234567890ee";
        let output = serialise(data);
        assert_eq!(output, expected_output);
    }
}
