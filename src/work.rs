use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

/// Piece-download work
pub struct Work {
    /// Index of piece
    index: u64,
    /// Length of piece
    length: u64,
    /// SHA1 hash of piece
    hash: Vec<u8>,
}

struct SharedQueueInner(VecDeque<Work>);

/// Shared queue containing piece-download work
#[derive(Clone)]
pub struct SharedQueue {
    inner: Arc<Mutex<SharedQueueInner>>,
}

impl SharedQueue {
    /// Create shared queue from vector of work
    pub fn new(work: Vec<Work>) -> SharedQueue {
        let queue = VecDeque::from(work);
        SharedQueue {
            inner: Arc::new(Mutex::new(SharedQueueInner(queue))),
        }
    }
}
