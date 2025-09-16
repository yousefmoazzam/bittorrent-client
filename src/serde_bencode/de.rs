use super::error::{Error, Result};
use nom::{
    bytes::tag,
    sequence::{delimited, terminated},
    Parser,
};

struct Deserialiser<'de> {
    /// Raw bytes of bencoded data being read, parsed, then deserialised
    input: &'de [u8],
}

impl<'de> Deserialiser<'de> {
    fn from_bytes(input: &'de [u8]) -> Deserialiser<'de> {
        Deserialiser { input }
    }
}

fn from_bytes<'a, T>(data: &'a [u8]) -> Result<T>
where
    T: serde::Deserialize<'a>,
{
    let mut deserialiser = Deserialiser::from_bytes(data);
    let deserialised_instance = T::deserialize(&mut deserialiser)?;
    if deserialiser.input.is_empty() {
        Ok(deserialised_instance)
    } else {
        Err(Error::LeftoverData)
    }
}

impl<'de> Deserialiser<'de> {
    /// Look at next byte in input but don't consume it
    fn peek_byte(&mut self) -> Result<u8> {
        match self.input.first() {
            Some(val) => Ok(*val),
            None => Err(Error::Eof),
        }
    }

    /// Look at next byte in input and consume it
    fn next_byte(&mut self) -> Result<u8> {
        let first = self.input.first().ok_or(Error::Eof)?;
        self.input = &self.input[1..];
        Ok(*first)
    }

    fn parse_integer(&mut self) -> Result<i64> {
        match delimited(
            tag("i"),
            nom::character::complete::i64::<&[u8], nom::error::Error<&[u8]>>,
            tag("e"),
        )
        .parse(self.input)
        {
            Ok((leftover, val)) => {
                self.input = leftover;
                Ok(val)
            }
            Err(_) => Err(Error::Message("Failed to parse integer".to_string())),
        }
    }

    fn parse_byte_string(&mut self) -> Result<&[u8]> {
        match terminated(
            nom::character::complete::i64::<&[u8], nom::error::Error<&[u8]>>,
            tag(":"),
        )
        .parse(self.input)
        {
            Ok((leftover, len)) => {
                self.input = &leftover[len as usize..];
                Ok(&leftover[..len as usize])
            }
            Err(_) => Err(Error::Message("Failed to parse byte string".to_string())),
        }
    }
}

impl<'de, 'a> serde::de::Deserializer<'de> for &'a mut Deserialiser<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        let next_byte = self.peek_byte()?;

        if next_byte == b'i' {
            return self.deserialize_i64(visitor);
        }

        if next_byte == b'l' {
            return self.deserialize_seq(visitor);
        }

        if next_byte == b'd' {
            return self.deserialize_map(visitor);
        }

        match next_byte {
            b'0' | b'1' | b'2' | b'3' | b'4' | b'5' | b'6' | b'7' | b'8' | b'9' => {
                self.deserialize_bytes(visitor)
            }
            _ => Err(Error::Message(format!(
                "Unexpected byte value: {next_byte}"
            ))),
        }
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_i8<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_i8(self.parse_integer()? as i8)
    }

    fn deserialize_i16<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_i16(self.parse_integer()? as i16)
    }

    fn deserialize_i32<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_i32(self.parse_integer()? as i32)
    }

    fn deserialize_i64<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_i64(self.parse_integer()?)
    }

    fn deserialize_i128<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_i128(self.parse_integer()? as i128)
    }

    fn deserialize_u8<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_u8(self.parse_integer()? as u8)
    }

    fn deserialize_u16<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_u16(self.parse_integer()? as u16)
    }

    fn deserialize_u32<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_u32(self.parse_integer()? as u32)
    }

    fn deserialize_u64<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_u64(self.parse_integer()? as u64)
    }

    fn deserialize_u128<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_u128(self.parse_integer()? as u128)
    }

    fn deserialize_bool<V>(self, _visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_f32<V>(self, _visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_f64<V>(self, _visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_bytes<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_bytes(self.parse_byte_string()?)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_byte_buf(self.parse_byte_string()?.to_vec())
    }

    fn deserialize_str<V>(self, _visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_string<V>(self, _visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_seq<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        if self.peek_byte()? == b'l' {
            // Move past `l` character that signals start of list
            self.input = &self.input[1..];
            let elem = visitor.visit_seq(SeqElement::new(self))?;
            // Parse `e` character that signals end of list
            if self.next_byte()? == b'e' {
                Ok(elem)
            } else {
                Err(Error::ExpectedListEnd)
            }
        } else {
            Err(Error::ExpectedList)
        }
    }

    fn deserialize_tuple<V>(
        self,
        _len: usize,
        _visitor: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_map<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        if self.peek_byte()? == b'd' {
            // Move past `d` character that signals start of map
            self.input = &self.input[1..];
            let elem = visitor.visit_map(SeqElement::new(self))?;
            // Parse `e` character that signals end of map
            if self.next_byte()? == b'e' {
                Ok(elem)
            } else {
                Err(Error::ExpectedMapEnd)
            }
        } else {
            Err(Error::ExpectedMap)
        }
    }

    fn deserialize_char<V>(self, _visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_unit<V>(self, _visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        _visitor: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_option<V>(self, _visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_identifier<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        _visitor: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        _visitor: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        _visitor: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
    }
}

struct SeqElement<'a, 'de: 'a> {
    deserialiser: &'a mut Deserialiser<'de>,
}

impl<'a, 'de> SeqElement<'a, 'de> {
    fn new(deserialiser: &'a mut Deserialiser<'de>) -> SeqElement<'a, 'de> {
        SeqElement { deserialiser }
    }
}

impl<'de, 'a> serde::de::SeqAccess<'de> for SeqElement<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T>(
        &mut self,
        seed: T,
    ) -> std::result::Result<Option<T::Value>, Self::Error>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        if self.deserialiser.peek_byte()? == b'e' {
            return Ok(None);
        }

        seed.deserialize(&mut *self.deserialiser).map(Some)
    }
}

impl<'de, 'a> serde::de::MapAccess<'de> for SeqElement<'a, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> std::result::Result<Option<K::Value>, Self::Error>
    where
        K: serde::de::DeserializeSeed<'de>,
    {
        if self.deserialiser.peek_byte()? == b'e' {
            return Ok(None);
        }

        seed.deserialize(&mut *self.deserialiser).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        seed.deserialize(&mut *self.deserialiser)
    }
}

#[cfg(test)]
mod tests {
    use crate::serde_bencode::de::from_bytes;
    use serde::{Deserialize, Serialize};

    #[test]
    fn integer_in_struct() {
        #[derive(Debug, PartialEq, Deserialize, Serialize)]
        struct Foo {
            bar: i64,
        }

        let integer = 2;
        let bytes = b"d3:bari2ee";
        let expected = Foo { bar: integer };
        assert_eq!(expected, from_bytes(bytes).unwrap());
    }

    #[test]
    fn list_in_struct() {
        #[derive(Deserialize, Serialize, Debug, PartialEq)]
        struct Foo {
            bar: Vec<u8>,
        }

        let bytes = b"d3:barli1ei34eee";
        let expected = Foo { bar: vec![1, 34] };
        assert_eq!(expected, from_bytes(bytes).unwrap())
    }

    #[test]
    fn byte_string_in_struct() {
        #[derive(Deserialize, Serialize, Debug, PartialEq)]
        struct Foo {
            #[serde(with = "serde_bytes")]
            bar: Vec<u8>,
        }

        let bytes = b"d3:bar5:helloe";
        let expected = Foo {
            bar: "hello".as_bytes().to_vec(),
        };
        assert_eq!(expected, from_bytes(bytes).unwrap())
    }
}
