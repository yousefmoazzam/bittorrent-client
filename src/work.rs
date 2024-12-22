use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

/// Piece-download work
pub struct Work {
    /// Index of piece
    pub index: u64,
    /// Length of piece
    pub length: u64,
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
    /// Create shared queue from vector of work
    pub fn new(work: Vec<Work>) -> SharedQueue {
        let queue = VecDeque::from(work);
        SharedQueue {
            inner: Arc::new(Mutex::new(SharedQueueInner(queue))),
        }
    }

    /// Get work element from front of queue
    pub fn dequeue(&self) -> Option<Work> {
        let mut lock = self.inner.lock().unwrap();
        lock.0.pop_front()
    }

    /// Put work element onto back of queue
    pub fn enqueue(&self, work: Work) {
        let mut lock = self.inner.lock().unwrap();
        lock.0.push_back(work);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn tasks_can_get_work_from_queue() {
        let indices = (0..16).collect::<Vec<u64>>();

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

        let task_one_handle = tokio::spawn(async move {
            while let Some(work) = queue.dequeue() {
                task_one_work_indices.push(work.index);
                // Short sleep to enable interleaving of tasks
                tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
            }
            task_one_work_indices
        });

        let task_two_handle = tokio::spawn(async move {
            while let Some(work) = queue_handle1.dequeue() {
                task_two_work_indices.push(work.index);
                // Short sleep to enable interleaving of tasks
                tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
            }
            task_two_work_indices
        });

        let mut task_one_work_indices = task_one_handle.await.unwrap();
        let mut task_two_work_indices = task_two_handle.await.unwrap();
        task_one_work_indices.append(&mut task_two_work_indices);
        task_one_work_indices.sort();
        assert_eq!(indices, task_one_work_indices);
    }

    #[tokio::test]
    async fn tasks_can_put_work_onto_queue() {
        let indices = (0..16).collect::<Vec<u64>>();
        let extra_indices = (16..24).collect::<Vec<u64>>();
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

        let task_one_handle = tokio::spawn(async move {
            while let Some(work) = queue.dequeue() {
                task_one_work_indices.push(work.index);
                // Short sleep to enable interleaving of tasks
                tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
            }
            task_one_work_indices
        });

        let task_two_handle = tokio::spawn(async move {
            while let Some(work) = queue_handle1.dequeue() {
                task_two_work_indices.push(work.index);
                // Short sleep to enable interleaving of tasks
                tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
            }
            task_two_work_indices
        });

        let extra_work = extra_indices
            .iter()
            .map(|index| Work {
                index: *index,
                length: 32,
                hash: vec![0x04],
            })
            .collect::<Vec<Work>>();
        let task_three_handle = tokio::spawn(async move {
            for work in extra_work {
                queue_handle2.enqueue(work);
                // Short sleep to enable interleaving of tasks
                tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
            }
        });
        let mut task_one_work_indices = task_one_handle.await.unwrap();
        let mut task_two_work_indices = task_two_handle.await.unwrap();
        task_three_handle.await.unwrap();
        task_one_work_indices.append(&mut task_two_work_indices);
        task_one_work_indices.sort();
        assert_eq!(expected_indices, task_one_work_indices);
    }
}
