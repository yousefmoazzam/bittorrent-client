use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

/// Piece-download work
pub struct Work {
    /// Index of piece
    pub index: u32,
    /// Length of piece
    pub length: u32,
    /// SHA1 hash of piece
    pub hash: Vec<u8>,
}

struct SharedQueueInner(VecDeque<Work>);

/// Shared queue containing piece-download work
#[derive(Clone)]
pub struct SharedQueue {
    inner: Arc<Mutex<SharedQueueInner>>,
}

impl SharedQueue {
    /// Create shared queue from work
    pub fn new(work: impl Into<VecDeque<Work>>) -> SharedQueue {
        SharedQueue {
            inner: Arc::new(Mutex::new(SharedQueueInner(work.into()))),
        }
    }

    /// Get work element from front of queue
    pub fn dequeue(&self) -> Option<Work> {
        let mut lock = self.inner.lock().unwrap();
        lock.0.pop_front()
    }

    /// Put work element onto front of queue
    pub fn enqueue(&self, work: Work) {
        let mut lock = self.inner.lock().unwrap();
        lock.0.push_front(work);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn tasks_can_get_work_from_queue() {
        let indices = (0..16).collect::<Vec<u32>>();

        let work = indices
            .iter()
            .map(|index| Work {
                index: *index,
                length: 16,
                hash: vec![0x01],
            })
            .collect::<Vec<Work>>();
        let queue = SharedQueue::new(work);
        let queue_handle1 = queue.clone();
        let mut task_one_work_indices = Vec::new();
        let mut task_two_work_indices = Vec::new();

        let fut_one = async move {
            while let Some(work) = queue.dequeue() {
                task_one_work_indices.push(work.index);
                // Yield to enable interleaving of tasks
                tokio::task::yield_now().await;
            }
            task_one_work_indices
        };
        let fut_two = async move {
            while let Some(work) = queue_handle1.dequeue() {
                task_two_work_indices.push(work.index);
                // Yield to enable interleaving of tasks
                tokio::task::yield_now().await;
            }
            task_two_work_indices
        };
        let (mut task_one_work_indices, mut task_two_work_indices) = tokio::join!(fut_one, fut_two);
        task_one_work_indices.append(&mut task_two_work_indices);
        task_one_work_indices.sort();
        assert_eq!(indices, task_one_work_indices);
    }

    #[tokio::test]
    async fn tasks_can_put_work_onto_queue() {
        let indices = (0..16).collect::<Vec<u32>>();
        let extra_indices = (16..24).collect::<Vec<u32>>();
        let mut expected_indices = indices.clone();
        expected_indices.append(&mut extra_indices.clone());

        let original_work = indices
            .iter()
            .map(|index| Work {
                index: *index,
                length: 16,
                hash: vec![0x01],
            })
            .collect::<Vec<Work>>();
        let queue = SharedQueue::new(original_work);
        let queue_handle1 = queue.clone();
        let queue_handle2 = queue.clone();
        let mut task_one_work_indices = Vec::new();
        let mut task_two_work_indices = Vec::new();

        let fut_one = async move {
            while let Some(work) = queue.dequeue() {
                task_one_work_indices.push(work.index);
                // Yield to enable interleaving of tasks
                tokio::task::yield_now().await;
            }
            task_one_work_indices
        };
        let fut_two = async move {
            while let Some(work) = queue_handle1.dequeue() {
                task_two_work_indices.push(work.index);
                // Yield to enable interleaving of tasks
                tokio::task::yield_now().await;
            }
            task_two_work_indices
        };
        let extra_work = extra_indices
            .iter()
            .map(|index| Work {
                index: *index,
                length: 32,
                hash: vec![0x04],
            })
            .collect::<Vec<Work>>();
        let fut_three = async move {
            for work in extra_work {
                queue_handle2.enqueue(work);
                // Yield to enable interleaving of tasks
                tokio::task::yield_now().await;
            }
        };

        let (mut task_one_work_indices, mut task_two_work_indices, _) =
            tokio::join!(fut_one, fut_two, fut_three);
        task_one_work_indices.append(&mut task_two_work_indices);
        task_one_work_indices.sort();
        assert_eq!(expected_indices, task_one_work_indices);
    }
}
