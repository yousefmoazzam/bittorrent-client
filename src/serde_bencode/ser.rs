use serde::Serialize;

use super::error::{Error, Result};

const COLON_ASCII: u8 = 58;
const L_ASCII: u8 = 108;
const E_ASCII: u8 = 101;
const D_ASCII: u8 = 100;

struct Serialiser {
    output: Vec<u8>,
}

fn to_bytes<T: serde::Serialize>(value: &T) -> Result<Vec<u8>> {
    let mut serialiser = Serialiser { output: Vec::new() };
    value.serialize(&mut serialiser)?;
    Ok(serialiser.output)
}

impl<'a> serde::ser::Serializer for &'a mut Serialiser {
    /// Tutorial says that if a serialiaser produces text or binary output, to return the unit
    /// tuple and to put the serilisaiton output into a value stored in the serialiaser, which is
    /// what the `self.output` is for
    type Ok = ();

    /// Using the custom error type we created for representing potential errors in
    /// serialisaiotn/deserialisaiton to/from the bencode data format
    type Error = Error;

    /// There's no need to store state in addition to the `self.output` in order to perform
    /// serialisaiotn, so the assoicated types available for storing additional state can be set to
    /// `Self` (I geuss that's sopme default for indicating that you don;t need the associated
    /// type? Would maybe think to use unit tuple, but ok, do what the tutoriual does for now
    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    /// No boolean value in bencode, serialise true as 1 and false as 0
    fn serialize_bool(self, v: bool) -> std::result::Result<Self::Ok, Self::Error> {
        let string = if v {
            "i1e".to_string()
        } else {
            "i0e".to_string()
        };
        self.output.append(&mut string.as_bytes().to_vec());
        Ok(())
    }

    /// Bencode data assumes i64, so upcast to i64 before serialising
    fn serialize_i8(self, v: i8) -> std::result::Result<Self::Ok, Self::Error> {
        self.serialize_i64(i64::from(v))
    }

    /// Bencode data assumes i64, so upcast to i64 before serialising
    fn serialize_i16(self, v: i16) -> std::result::Result<Self::Ok, Self::Error> {
        self.serialize_i64(i64::from(v))
    }

    /// Bencode data assumes i64, so upcast to i64 before serialising
    fn serialize_i32(self, v: i32) -> std::result::Result<Self::Ok, Self::Error> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i64(self, v: i64) -> std::result::Result<Self::Ok, Self::Error> {
        let string = format!("i{v}e");
        self.output.append(&mut string.as_bytes().to_vec());
        Ok(())
    }

    /// Bencode data assumes i64, let's just upcats to u64 then serialise
    fn serialize_u8(self, v: u8) -> std::result::Result<Self::Ok, Self::Error> {
        self.serialize_u64(u64::from(v))
    }

    /// Bencode data assumes i64, let's just upcats to u64 then serialise
    fn serialize_u16(self, v: u16) -> std::result::Result<Self::Ok, Self::Error> {
        self.serialize_u64(u64::from(v))
    }

    /// Bencode data assumes i64, let's just upcats to u64 then serialise
    fn serialize_u32(self, v: u32) -> std::result::Result<Self::Ok, Self::Error> {
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u64(self, v: u64) -> std::result::Result<Self::Ok, Self::Error> {
        let string = format!("i{v}e");
        self.output.append(&mut string.as_bytes().to_vec());
        Ok(())
    }

    /// Upcate to f64 and then serialise
    fn serialize_f32(self, v: f32) -> std::result::Result<Self::Ok, Self::Error> {
        self.serialize_f64(f64::from(v))
    }

    /// Bencode data doesn;t support flaoting point data, so convert to a u64 and throw away the
    /// fraciotnal part?
    fn serialize_f64(self, v: f64) -> std::result::Result<Self::Ok, Self::Error> {
        self.serialize_u64(v as u64)
    }

    /// Serialise as a single ASCII encoded character
    fn serialize_char(self, v: char) -> std::result::Result<Self::Ok, Self::Error> {
        let string = format!("1:{v}");
        self.output
            .append(&mut string.to_string().as_bytes().to_vec());
        Ok(())
    }

    /// Serialise as a byte string
    fn serialize_str(self, v: &str) -> std::result::Result<Self::Ok, Self::Error> {
        let len = v.len();
        let string = format!("{len}:{v}");
        self.output.append(&mut string.as_bytes().to_vec());
        Ok(())
    }

    /// Serialise as a byte string
    fn serialize_bytes(self, v: &[u8]) -> std::result::Result<Self::Ok, Self::Error> {
        self.output.push(v.len() as u8);
        self.output.push(COLON_ASCII);
        self.output.append(&mut v.to_vec());
        Ok(())
    }

    /// Serialise as a boolean, false
    fn serialize_none(self) -> std::result::Result<Self::Ok, Self::Error> {
        self.serialize_bool(false)
    }

    /// Serialise as simply the contained value (ie, losing informaiton that we had a `Some`
    /// variant of a `Result`
    fn serialize_some<T>(self, value: &T) -> std::result::Result<Self::Ok, Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        value.serialize(self)
    }

    /// Serialise as boolean, false
    fn serialize_unit(self) -> std::result::Result<Self::Ok, Self::Error> {
        self.serialize_bool(false)
    }

    /// Serialise as boolean, false
    fn serialize_unit_struct(
        self,
        _name: &'static str,
    ) -> std::result::Result<Self::Ok, Self::Error> {
        self.serialize_bool(false)
    }

    /// Serialise as byte string
    ///
    /// Note that this is a variant of an enum which has no extra data attached to it, so all we
    /// have is a name for the enum type it';s a part of, the name of the variant itself, and the
    /// index of the variant within the enum
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> std::result::Result<Self::Ok, Self::Error> {
        self.serialize_str(variant)
    }

    /// Serialise as the contained value, throwing away the newtype wrapper struct
    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        value: &T,
    ) -> std::result::Result<Self::Ok, Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        value.serialize(self)
    }

    /// FYI, this is for an enum type which has a single variant that has a tuple with a single
    /// element, like the following:
    /// ```rust
    /// enum Foo {
    ///     Bar(u8),
    /// }
    ///
    /// ```
    ///
    /// As the mpa has only one key-value pair, can do the logic more easily rather than calling
    /// the map-serialisation method.
    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> std::result::Result<Self::Ok, Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        self.output.push(D_ASCII);
        variant.serialize(&mut *self)?;
        value.serialize(&mut *self)?;
        self.output.push(E_ASCII);
        Ok(())
    }

    /// Seriliase the `l` character to signify the start of the list
    fn serialize_seq(
        self,
        _len: Option<usize>,
    ) -> std::result::Result<Self::SerializeSeq, Self::Error> {
        self.output.push(L_ASCII);
        Ok(self)
    }

    /// Serialise as a sequence (ie, as a list)
    fn serialize_tuple(self, len: usize) -> std::result::Result<Self::SerializeTuple, Self::Error> {
        self.serialize_seq(Some(len))
    }

    /// Serialise as a tuple (ie, as a list)
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> std::result::Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_seq(Some(len))
    }

    /// Serialising a variant of an enum which has a payload of a tuple, like the following:
    ///
    /// ```rust
    /// enum Foo {
    ///     Bar(u8, bool, i32),
    /// }
    /// ```
    ///
    /// Start creating the map, and then let the list-serialisaiton take over and compleet
    /// serialising the lsit, and then finally let the tuple variant trait serialisaiotn
    /// impleemntaiotn finish the serilisaiotn off (ie, add the end chars).
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> std::result::Result<Self::SerializeTupleVariant, Self::Error> {
        self.output.push(D_ASCII);
        variant.serialize(&mut *self)?;
        self.output.push(COLON_ASCII);
        self.output.push(L_ASCII);
        Ok(self)
    }

    /// Serialise the `d` character, to signifit the start of a map
    fn serialize_map(
        self,
        _len: Option<usize>,
    ) -> std::result::Result<Self::SerializeMap, Self::Error> {
        self.output.push(D_ASCII);
        Ok(self)
    }

    /// Serialise as a map
    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> std::result::Result<Self::SerializeStruct, Self::Error> {
        self.serialize_map(Some(len))
    }

    /// Serialise a variant of an enum which has a payload of a struct, like the following:
    /// ```rust
    /// enum Foo {
    ///     Baz { a: u8, b: bool },
    /// }
    /// ```
    ///
    /// Start creating the map, and then let the struct-serialisaiton take over and compleet
    /// serialising the struct, and then finally let the struct variant trait serialisaiotn
    /// impleemntaiotn finish the serilisaiotn off (ie, add the end chars).
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> std::result::Result<Self::SerializeStructVariant, Self::Error> {
        self.output.push(D_ASCII);
        variant.serialize(&mut *self)?;
        self.output.push(D_ASCII);
        Ok(self)
    }
}

impl<'a> serde::ser::SerializeSeq for &'a mut Serialiser {
    type Ok = ();
    type Error = Error;

    /// Serialise a single element in the list
    ///
    /// Note: there's no delimter between elements in a list, so only need to serialise the value
    /// and that's it
    fn serialize_element<T>(&mut self, value: &T) -> std::result::Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        value.serialize(&mut **self)
    }

    /// Serialise the character `e` to signify the end of the list
    fn end(self) -> std::result::Result<Self::Ok, Self::Error> {
        self.output.push(E_ASCII);
        Ok(())
    }
}

impl<'a> serde::ser::SerializeTuple for &'a mut Serialiser {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> std::result::Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> std::result::Result<Self::Ok, Self::Error> {
        self.output.push(E_ASCII);
        Ok(())
    }
}

impl<'a> serde::ser::SerializeTupleStruct for &'a mut Serialiser {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> std::result::Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> std::result::Result<Self::Ok, Self::Error> {
        self.output.push(E_ASCII);
        Ok(())
    }
}

impl<'a> serde::ser::SerializeTupleVariant for &'a mut Serialiser {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> std::result::Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    /// Finish off serialising the list, as well as the enclosing map
    fn end(self) -> std::result::Result<Self::Ok, Self::Error> {
        self.output.push(E_ASCII);
        self.output.push(E_ASCII);
        Ok(())
    }
}

impl<'a> serde::ser::SerializeMap for &'a mut Serialiser {
    type Ok = ();
    type Error = Error;

    /// TODO: Make sre that the key is onyl a byte string, it shoudln;t be any other type
    fn serialize_key<T>(&mut self, key: &T) -> std::result::Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        key.serialize(&mut **self)
    }

    fn serialize_value<T>(&mut self, value: &T) -> std::result::Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> std::result::Result<Self::Ok, Self::Error> {
        self.output.push(E_ASCII);
        Ok(())
    }
}

impl<'a> serde::ser::SerializeStruct for &'a mut Serialiser {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> std::result::Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        key.serialize(&mut **self)?;
        value.serialize(&mut **self)
    }

    fn end(self) -> std::result::Result<Self::Ok, Self::Error> {
        self.output.push(E_ASCII);
        Ok(())
    }
}

impl<'a> serde::ser::SerializeStructVariant for &'a mut Serialiser {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> std::result::Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        key.serialize(&mut **self)?;
        value.serialize(&mut **self)
    }

    /// Need to finish serilsing the map that's the struct payload opf the varian,t but also the
    /// map that's the top-level map contanin ghr single key-value pair
    fn end(self) -> std::result::Result<Self::Ok, Self::Error> {
        self.output.push(E_ASCII);
        self.output.push(E_ASCII);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::serde_bencode::ser::to_bytes;

    #[test]
    fn newtype_struct_wrapping_byte_string() {
        #[derive(serde::Serialize)]
        struct Foo(String);

        let my_str = "hello".to_string();
        let foo = Foo(my_str.clone());
        assert_eq!(
            to_bytes(&foo).unwrap(),
            format!("{}:{my_str}", my_str.len()).as_bytes()
        );
    }

    #[test]
    fn newtype_struct_wrapping_integer() {
        #[derive(serde::Serialize)]
        struct Foo(i64);

        let my_int = 10;
        let foo = Foo(my_int);
        assert_eq!(to_bytes(&foo).unwrap(), format!("i{my_int}e").as_bytes())
    }

    #[test]
    fn serialise_struct_as_map() {
        #[derive(serde::Serialize)]
        struct Foo {
            bar: i64,
            baz: bool,
        }

        let my_foo = Foo { bar: 2, baz: true };
        let expected = "d3:bari2e3:bazi1ee".as_bytes();
        assert_eq!(to_bytes(&my_foo).unwrap(), expected);
    }

    #[test]
    fn serialise_struct_as_map2() {
        #[derive(serde::Serialize)]
        struct Foo {
            bar: i64,
            baz: String,
        }

        let my_foo = Foo {
            bar: 2,
            baz: "hello".to_string(),
        };
        let expected = "d3:bari2e3:baz5:helloe".as_bytes();
        assert_eq!(to_bytes(&my_foo).unwrap(), expected);
    }
}
