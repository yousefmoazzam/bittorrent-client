use tokio::io::{AsyncRead, AsyncWrite};

use crate::client::Client;
use crate::message::Message;

/// Piece download worker
pub struct Worker<T: AsyncRead + AsyncWrite + Unpin> {
    client: Client<T>,
}

impl<T: AsyncRead + AsyncWrite + Unpin> Worker<T> {
    pub fn new(client: Client<T>) -> Worker<T> {
        Worker { client }
    }

    /// Download pieces from connected peer
    pub async fn download(&mut self) -> std::io::Result<()> {
        self.client.send(Message::Unchoke).await?;
        self.client.send(Message::Interested).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::handshake::Handshake;
    use crate::{PEER_ID, PSTR};

    use super::*;

    #[tokio::test]
    async fn download_sends_unchoke_and_interested_messages_to_peer() {
        let info_hash = (0x00..0x14).collect::<Vec<_>>();
        let their_peer_id = "-DEF123-efgh12345678";

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
            .write(&Message::Unchoke.serialise())
            .write(&Message::Interested.serialise())
            .build();
        let client = Client::new(mock_socket, info_hash, their_peer_id)
            .await
            .unwrap();
        let mut worker = Worker::new(client);
        let _ = worker.download().await;
    }
}
