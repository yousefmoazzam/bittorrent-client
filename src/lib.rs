use std::collections::HashMap;

pub mod client;
pub mod decode;
pub mod download;
pub mod encode;
pub mod handshake;
pub mod init;
pub mod message;
pub mod metainfo;
pub mod parse;
pub mod piece;
pub mod serialise;
pub mod torrent;
pub mod tracker;
pub mod work;
pub mod worker;

pub const HANDSHAKE_BYTES_LEN: usize = 68;
pub const PEER_ID: &str = "-ABC123-abcd12345678";
pub const PSTR: &str = "BitTorrent protocol";

#[derive(Debug, PartialEq)]
pub enum BencodeType {
    Integer(i64),
    ByteString(String),
    List(Vec<BencodeType>),
    Dict(HashMap<String, BencodeType>),
}
