use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader};

use crate::handshake::Handshake;
use crate::message::{Bitfield, Message};
use crate::PSTR;

const HANDSHAKE_BYTES_LEN: usize = 68;

/// Connected peer
pub struct Client;

impl Client {
    /// Send handshake and receive bitfield message from peer
    pub async fn new<T>(mut socket: T, info_hash: Vec<u8>, peer_id: &str) -> std::io::Result<()>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        Client::handshake(&mut socket, info_hash, peer_id).await?;
        Client::receive_bitfield(&mut socket).await?;
        Ok(())
    }

    /// Send initial handshake to peer
    async fn handshake<T>(mut socket: T, info_hash: Vec<u8>, peer_id: &str) -> std::io::Result<()>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        let initial_handshake = Handshake::new(PSTR.to_string(), info_hash, peer_id.into());
        socket.write_all(&initial_handshake.serialise()[..]).await?;

        let mut reader = BufReader::new(socket);
        let mut response_handshake = [0; HANDSHAKE_BYTES_LEN];
        reader.read_exact(&mut response_handshake[..]).await?;

        let deserialised_response = Handshake::deserialise(&response_handshake[..]);
        match deserialised_response.info_hash == initial_handshake.info_hash {
            true => Ok(()),
            false => {
                let msg = format!(
                    "Info hash mismatch: us={}, peer={}",
                    initial_handshake
                        .info_hash
                        .iter()
                        .map(|val| format!("{:02x}", val))
                        .collect::<Vec<String>>()
                        .join(""),
                    deserialised_response
                        .info_hash
                        .iter()
                        .map(|val| format!("{:02x}", val))
                        .collect::<Vec<String>>()
                        .join(""),
                );
                Err(std::io::Error::other(msg))
            }
        }
    }

    /// Receive initial bitfield message from peer
    async fn receive_bitfield<T>(socket: &mut T) -> std::io::Result<Bitfield>
    where
        T: AsyncRead + Unpin,
    {
        match Message::new(socket).await? {
            Message::Bitfield(_) => todo!(),
            _ => Err(std::io::Error::other("First message not bitfield")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::PEER_ID;

    #[tokio::test]
    async fn return_error_if_incorrect_info_hash_in_handshake_response() {
        let info_hash = (0x00..0x14).collect::<Vec<_>>();
        let incorrect_info_hash = (0x01..0x15).collect::<Vec<_>>();
        let their_peer_id = "-DEF123-efgh12345678";

        let expected_initial_handshake =
            Handshake::new(PSTR.to_string(), info_hash.clone(), PEER_ID.into());
        let bad_response_handshake = Handshake::new(
            PSTR.to_string(),
            incorrect_info_hash.clone(),
            their_peer_id.into(),
        );

        let mock_socket = tokio_test::io::Builder::new()
            .write(&expected_initial_handshake.serialise())
            .read(&bad_response_handshake.serialise())
            .build();
        let res = Client::new(mock_socket, info_hash.clone(), PEER_ID).await;

        let expected_err_msg = format!(
            "Info hash mismatch: us={}, peer={}",
            info_hash
                .iter()
                .map(|val| format!("{:02x}", val))
                .collect::<Vec<String>>()
                .join(""),
            incorrect_info_hash
                .iter()
                .map(|val| format!("{:02x}", val))
                .collect::<Vec<String>>()
                .join(""),
        );
        assert!(res.is_err_and(|val| val.to_string() == expected_err_msg));
    }

    #[tokio::test]
    async fn return_error_if_first_message_is_not_a_bitfield() {
        let info_hash = (0x00..0x14).collect::<Vec<_>>();
        let their_peer_id = "-DEF123-efgh12345678";

        let initial_handshake = Handshake::new(PSTR.to_string(), info_hash.clone(), PEER_ID.into());
        let response_handshake =
            Handshake::new(PSTR.to_string(), info_hash.clone(), their_peer_id.into());
        let len: u32 = 1;
        let id = 0x03;
        let mut incorrect_first_message_data = u32::to_be_bytes(len).to_vec();
        incorrect_first_message_data.push(id);

        let mock_socket = tokio_test::io::Builder::new()
            .write(&initial_handshake.serialise())
            .read(&response_handshake.serialise())
            .read(&incorrect_first_message_data[..])
            .build();
        let res = Client::new(mock_socket, info_hash.clone(), PEER_ID).await;

        let expected_err_msg = "First message not bitfield";
        assert!(res.is_err_and(|val| val.to_string() == expected_err_msg));
    }
}
