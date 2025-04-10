use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::handshake::Handshake;
use crate::message::{Bitfield, Message};
use crate::{HANDSHAKE_BYTES_LEN, PEER_ID, PSTR};

/// Connected peer
pub struct Client<T: AsyncRead + AsyncWrite + Unpin> {
    /// Socket for peer communication
    socket: T,
    /// ID of connected peer
    pub peer_id: Vec<u8>,
    /// Whether the connection is choked or not
    pub choked: bool,
    /// Bitfield associated with peer
    pub bitfield: Bitfield,
    /// Info hash of the file
    pub info_hash: Vec<u8>,
}

impl<T> Client<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    /// Send handshake and receive bitfield message from peer
    pub async fn new(mut socket: T, info_hash: Vec<u8>) -> std::io::Result<Client<T>> {
        let peer_id = Client::handshake(&mut socket, info_hash.clone()).await?;
        let bitfield = Client::receive_bitfield(&mut socket).await?;
        Ok(Client {
            socket,
            peer_id,
            choked: true,
            bitfield,
            info_hash,
        })
    }

    /// Send initial handshake to peer
    async fn handshake(mut socket: T, info_hash: Vec<u8>) -> std::io::Result<Vec<u8>> {
        let initial_handshake = Handshake::new(PSTR.to_string(), info_hash, PEER_ID.into());
        socket.write_all(&initial_handshake.serialise()[..]).await?;
        let mut response_handshake = [0; HANDSHAKE_BYTES_LEN];
        socket.read_exact(&mut response_handshake[..]).await?;

        let deserialised_response = Handshake::deserialise(&response_handshake[..]);
        match deserialised_response.info_hash == initial_handshake.info_hash {
            true => Ok(deserialised_response.peer_id),
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
    async fn receive_bitfield(socket: &mut T) -> std::io::Result<Bitfield> {
        match Message::deserialise(socket).await? {
            Message::Bitfield(bitfield) => Ok(bitfield),
            _ => Err(std::io::Error::other("First message not bitfield")),
        }
    }

    /// Receive message from peer
    pub async fn receive(&mut self) -> std::io::Result<Message> {
        let message = Message::deserialise(&mut self.socket).await?;
        match message {
            Message::Unchoke => {
                self.choked = false;
                Ok(message)
            }
            Message::Choke => {
                self.choked = true;
                Ok(message)
            }
            _ => Ok(message),
        }
    }

    /// Send message to peer
    pub async fn send(&mut self, message: Message) -> std::io::Result<()> {
        self.socket.write_all(&message.serialise()).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

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
        let res = Client::new(mock_socket, info_hash.clone()).await;

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
        let res = Client::new(mock_socket, info_hash.clone()).await;

        let expected_err_msg = "First message not bitfield";
        assert!(res.is_err_and(|val| val.to_string() == expected_err_msg));
    }

    #[tokio::test]
    async fn return_client_if_successful_handshake_and_receive_bitifield_message() {
        let info_hash = (0x00..0x14).collect::<Vec<_>>();
        let their_peer_id = b"-DEF123-efgh12345678";

        let initial_handshake = Handshake::new(PSTR.to_string(), info_hash.clone(), PEER_ID.into());
        let response_handshake =
            Handshake::new(PSTR.to_string(), info_hash.clone(), their_peer_id.into());
        let len: u32 = 2;
        let id = 0x05;
        let payload = vec![0x10];
        let mut bitfield_message = u32::to_be_bytes(len).to_vec();
        bitfield_message.push(id);
        bitfield_message.append(&mut payload.clone());
        let mock_socket = tokio_test::io::Builder::new()
            .write(&initial_handshake.serialise())
            .read(&response_handshake.serialise())
            .read(&bitfield_message)
            .build();
        let client = Client::new(mock_socket, info_hash.clone()).await.unwrap();

        let expected_bitfield = Bitfield::new(payload);
        assert!(client.choked);
        assert_eq!(client.peer_id, their_peer_id);
        assert_eq!(client.info_hash, info_hash);
        assert_eq!(client.bitfield, expected_bitfield);
    }

    #[test]
    fn return_client_if_successful_handshake_and_receive_bitifield_message_real_socket() {
        let info_hash = (0x00..0x14).collect::<Vec<_>>();
        let their_peer_id = b"-DEF123-efgh12345678";

        let response_handshake =
            Handshake::new(PSTR.to_string(), info_hash.clone(), their_peer_id.into());
        let len: u32 = 2;
        let id = 0x05;
        let payload = vec![0x10];
        let mut bitfield_message = u32::to_be_bytes(len).to_vec();
        bitfield_message.push(id);
        bitfield_message.append(&mut payload.clone());

        let addr = "127.0.0.1:13245";
        let listener = std::net::TcpListener::bind(addr).unwrap();

        let handle = std::thread::spawn(move || {
            let mut throwaway_buf = [0; HANDSHAKE_BYTES_LEN];
            let (mut socket, _) = listener.accept().unwrap();
            // Peer reads initial handshake from client
            socket.read_exact(&mut throwaway_buf).unwrap();

            // Peer writes response handshake to client
            socket.write_all(&response_handshake.serialise()).unwrap();

            // Peer writes bitfield message to client
            socket.write_all(&bitfield_message).unwrap();
        });

        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let socket = tokio::net::TcpStream::connect(addr).await.unwrap();
                let client = Client::new(socket, info_hash.clone()).await.unwrap();
                handle.join().unwrap();

                let expected_bitfield = Bitfield::new(payload);
                assert!(client.choked);
                assert_eq!(client.peer_id, their_peer_id);
                assert_eq!(client.info_hash, info_hash);
                assert_eq!(client.bitfield, expected_bitfield);
            });
    }

    #[tokio::test]
    async fn client_receives_correct_message_from_peer() {
        let info_hash = (0x00..0x14).collect::<Vec<_>>();
        let their_peer_id = b"-DEF123-efgh12345678";

        let len: u32 = 1;
        let id = 0x01;
        let mut buf = u32::to_be_bytes(len).to_vec();
        buf.push(id);
        let expected_message = Message::Unchoke;
        let mock_socket = tokio_test::io::Builder::new().read(&buf).build();
        let mut client = Client {
            socket: mock_socket,
            peer_id: their_peer_id.to_vec(),
            choked: true,
            bitfield: Bitfield::new(vec![0x05]),
            info_hash,
        };
        assert!(client
            .receive()
            .await
            .is_ok_and(|msg| msg == expected_message));
    }

    #[tokio::test]
    async fn client_sends_correct_message_data_to_peer() {
        let info_hash = (0x00..0x14).collect::<Vec<_>>();
        let their_peer_id = b"-DEF123-efgh12345678";
        let message = Message::Unchoke;
        let len = 1;
        let id = 1;
        let mut expected_buf = u32::to_be_bytes(len).to_vec();
        expected_buf.push(id);
        let mock_socket = tokio_test::io::Builder::new().write(&expected_buf).build();
        let mut client = Client {
            socket: mock_socket,
            peer_id: their_peer_id.to_vec(),
            choked: true,
            bitfield: Bitfield::new(vec![0x05]),
            info_hash,
        };
        assert!(client.send(message).await.is_ok());
    }

    #[tokio::test]
    async fn client_sets_choked_false_if_peer_sends_unchoke_message() {
        let info_hash = (0x00..0x14).collect::<Vec<_>>();
        let their_peer_id = b"-DEF123-efgh12345678";

        let mock_socket = tokio_test::io::Builder::new()
            .read(&Message::Unchoke.serialise())
            .build();
        let mut client = Client {
            socket: mock_socket,
            peer_id: their_peer_id.to_vec(),
            choked: true,
            bitfield: Bitfield::new(vec![0x05]),
            info_hash,
        };
        client.receive().await.unwrap();
        assert!(!client.choked);
    }

    #[tokio::test]
    async fn client_sets_choked_true_if_peer_sends_choke_message() {
        let info_hash = (0x00..0x14).collect::<Vec<_>>();
        let their_peer_id = b"-DEF123-efgh12345678";

        let mock_socket = tokio_test::io::Builder::new()
            .read(&Message::Unchoke.serialise())
            .read(&Message::Choke.serialise())
            .build();
        let mut client = Client {
            socket: mock_socket,
            peer_id: their_peer_id.to_vec(),
            choked: true,
            bitfield: Bitfield::new(vec![0x05]),
            info_hash,
        };
        client.receive().await.unwrap();
        client.receive().await.unwrap();
        assert!(client.choked);
    }
}
