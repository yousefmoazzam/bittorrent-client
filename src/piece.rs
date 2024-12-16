use tokio::sync::mpsc::Receiver;

/// Downloaded piece
#[derive(Debug)]
pub struct Piece {
    /// Index of piece within file
    index: u64,
    /// Piece data
    buf: Vec<u8>,
}

/// Receive completed pieces and store them in an in-memory buffer
pub async fn receiver(buf: &mut [u8], piece_length: usize, mut rx: Receiver<Piece>) {
    while let Some(piece_result) = rx.recv().await {
        let start = piece_result.index as usize * piece_length;
        let end = start + piece_length;
        buf[start..end].copy_from_slice(&piece_result.buf)
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

        tokio::spawn(async move {
            tx.send(Piece {
                index: 1,
                buf: pieces_first_half[1].to_vec(),
            })
            .await
            .unwrap();
            tx.send(Piece {
                index: 3,
                buf: pieces_first_half[3].to_vec(),
            })
            .await
            .unwrap();
            tx.send(Piece {
                index: 0,
                buf: pieces_first_half[0].to_vec(),
            })
            .await
            .unwrap();
            tx.send(Piece {
                index: 2,
                buf: pieces_first_half[2].to_vec(),
            })
            .await
            .unwrap();
        });

        tokio::spawn(async move {
            tx1.send(Piece {
                index: 1 + 4,
                buf: pieces_second_half[1].to_vec(),
            })
            .await
            .unwrap();
            tx1.send(Piece {
                index: 3 + 4,
                buf: pieces_second_half[3].to_vec(),
            })
            .await
            .unwrap();
            tx1.send(Piece {
                index: 4,
                buf: pieces_second_half[0].to_vec(),
            })
            .await
            .unwrap();
            tx1.send(Piece {
                index: 2 + 4,
                buf: pieces_second_half[2].to_vec(),
            })
            .await
            .unwrap();
        });

        receiver(&mut receiver_buf, PIECE_LEN, rx).await;
        assert_eq!(&receiver_buf[..], &original_data[..]);
    }
}