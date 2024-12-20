use std::collections::HashMap;

pub mod client;
pub mod decode;
pub mod encode;
pub mod handshake;
pub mod message;
pub mod metainfo;
pub mod piece;
pub mod torrent;
pub mod tracker;
pub mod work;

pub const PEER_ID: &str = "-ABC123-abcd12345678";
pub const PSTR: &str = "BitTorrent protocol";

#[derive(Debug, PartialEq)]
pub enum BencodeType {
    Integer(i64),
    ByteString(String),
    List(Vec<BencodeType>),
    Dict(HashMap<String, BencodeType>),
}
