use std::collections::HashMap;

pub mod decode;
pub mod handshake;
pub mod message;
pub mod metainfo;
pub mod tracker;

#[derive(Debug, PartialEq)]
pub enum BencodeType {
    Integer(i64),
    ByteString(String),
    List(Vec<BencodeType>),
    Dict(HashMap<String, BencodeType>),
}
