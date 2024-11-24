use std::collections::HashMap;

pub mod decode;

#[derive(Debug, PartialEq)]
pub enum BencodeType {
    Integer(i64),
    ByteString(String),
    List(Vec<BencodeType>),
    Dict(HashMap<String, BencodeType>),
}
