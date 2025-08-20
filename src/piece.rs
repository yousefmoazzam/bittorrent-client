use tokio::sync::mpsc::Receiver;

/// Downloaded piece
#[derive(Debug)]
pub struct Piece {
    /// Index of piece within file
    pub index: u32,
    /// Piece data
    pub buf: Vec<u8>,
}

/// Receive completed pieces and store them in an in-memory buffer
pub async fn receiver(
    buf: &mut [u8],
    piece_length: usize,
    mut rx: Receiver<Piece>,
    no_of_pieces: usize,
    completion_sender: tokio::sync::watch::Sender<bool>,
) {
    let mut downloaded_pieces = 0;
    while let Some(piece_result) = rx.recv().await {
        let start = piece_result.index as usize * piece_length;
        let end = if start + piece_length <= buf.len() {
            start + piece_length
        } else {
            buf.len()
        };
        buf[start..end].copy_from_slice(&piece_result.buf);
        downloaded_pieces += 1;
        if downloaded_pieces == no_of_pieces {
            completion_sender.send(true).unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn receiver_puts_pieces_together_correctly() {
        const PIECE_LEN: usize = 128;
        const CHANNEL_BUFFER_SIZE: usize = 8;

        let original_data = (0..PIECE_LEN as u8).collect::<Vec<u8>>().repeat(8);
        let mut pieces_first_half = Vec::new();
        let mut pieces_second_half = Vec::new();
        let mut count = 0;
        while count < original_data.len() / 2 {
            pieces_first_half.push(original_data[count..count + PIECE_LEN].to_vec());
            count += PIECE_LEN;
        }
        while count < original_data.len() {
            pieces_second_half.push(original_data[count..count + PIECE_LEN].to_vec());
            count += PIECE_LEN;
        }

        let mut receiver_buf = [0; 1024];
        let (tx, rx) = tokio::sync::mpsc::channel(CHANNEL_BUFFER_SIZE);
        let tx1 = tx.clone();
        let (completion_tx, _completion_rx) = tokio::sync::watch::channel(false);

        let sender_one_fut = async move {
            tokio::try_join!(
                tx.send(Piece {
                    index: 1,
                    buf: pieces_first_half[1].to_vec(),
                }),
                tx.send(Piece {
                    index: 3,
                    buf: pieces_first_half[3].to_vec(),
                }),
                tx.send(Piece {
                    index: 0,
                    buf: pieces_first_half[0].to_vec(),
                }),
                tx.send(Piece {
                    index: 2,
                    buf: pieces_first_half[2].to_vec(),
                })
            )
            .unwrap()
        };

        let sender_two_fut = async move {
            tokio::try_join!(
                tx1.send(Piece {
                    index: 1 + 4,
                    buf: pieces_second_half[1].to_vec(),
                }),
                tx1.send(Piece {
                    index: 3 + 4,
                    buf: pieces_second_half[3].to_vec(),
                }),
                tx1.send(Piece {
                    index: 4,
                    buf: pieces_second_half[0].to_vec(),
                }),
                tx1.send(Piece {
                    index: 2 + 4,
                    buf: pieces_second_half[2].to_vec(),
                })
            )
            .unwrap()
        };

        tokio::join!(
            sender_one_fut,
            sender_two_fut,
            receiver(
                &mut receiver_buf,
                PIECE_LEN,
                rx,
                CHANNEL_BUFFER_SIZE,
                completion_tx,
            )
        );
        assert_eq!(&receiver_buf[..], &original_data[..]);
    }

    #[tokio::test]
    async fn receiver_handles_truncated_last_piece() {
        const PIECE_LEN: usize = 128;
        const TRUNCATED_PIECE_LEN: usize = PIECE_LEN - 4;
        const CHANNEL_BUFFER_SIZE: usize = 8;

        let mut original_data = (0..PIECE_LEN as u8).collect::<Vec<u8>>().repeat(8);
        let _ = original_data.split_off(original_data.len() - 4);
        let mut pieces_first_half = Vec::new();
        let mut pieces_second_half = Vec::new();
        let mut count = 0;
        while count < original_data.len() / 2 {
            pieces_first_half.push(original_data[count..count + PIECE_LEN].to_vec());
            count += PIECE_LEN;
        }
        while count < original_data.len() - TRUNCATED_PIECE_LEN {
            pieces_second_half.push(original_data[count..count + PIECE_LEN].to_vec());
            count += PIECE_LEN;
        }
        pieces_second_half.push(original_data[count..count + TRUNCATED_PIECE_LEN].to_vec());

        let mut receiver_buf = [0; PIECE_LEN * (CHANNEL_BUFFER_SIZE - 1) + TRUNCATED_PIECE_LEN];
        let (tx, rx) = tokio::sync::mpsc::channel(CHANNEL_BUFFER_SIZE);
        let tx1 = tx.clone();
        let (completion_tx, _completion_rx) = tokio::sync::watch::channel(false);

        let sender_one_fut = async move {
            tokio::try_join!(
                tx.send(Piece {
                    index: 1,
                    buf: pieces_first_half[1].to_vec(),
                }),
                tx.send(Piece {
                    index: 3,
                    buf: pieces_first_half[3].to_vec(),
                }),
                tx.send(Piece {
                    index: 0,
                    buf: pieces_first_half[0].to_vec(),
                }),
                tx.send(Piece {
                    index: 2,
                    buf: pieces_first_half[2].to_vec(),
                })
            )
            .unwrap()
        };

        let sender_two_fut = async move {
            tokio::try_join!(
                tx1.send(Piece {
                    index: 1 + 4,
                    buf: pieces_second_half[1].to_vec(),
                }),
                tx1.send(Piece {
                    index: 3 + 4,
                    buf: pieces_second_half[3].to_vec(),
                }),
                tx1.send(Piece {
                    index: 4,
                    buf: pieces_second_half[0].to_vec(),
                }),
                tx1.send(Piece {
                    index: 2 + 4,
                    buf: pieces_second_half[2].to_vec(),
                })
            )
            .unwrap()
        };

        tokio::join!(
            sender_one_fut,
            sender_two_fut,
            receiver(
                &mut receiver_buf,
                PIECE_LEN,
                rx,
                CHANNEL_BUFFER_SIZE,
                completion_tx,
            )
        );
        assert_eq!(&receiver_buf[..], &original_data[..]);
    }
}
