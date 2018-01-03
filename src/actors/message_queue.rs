use std::sync::{Mutex, Arc, Condvar};
use std::collections::VecDeque;

#[derive(Debug)]
pub enum MessageQueueError {
    Empty,
}

pub struct MessageQueue<T> {
    queue: Arc<Mutex<VecDeque<T>>>,
    sleeping: Arc<(Mutex<bool>, Condvar)>,
}

impl<T> MessageQueue<T> {
    pub fn new() -> MessageQueue<T> {
        MessageQueue {
            queue: Arc::new(Mutex::new(VecDeque::new())),
            sleeping: Arc::new((Mutex::new(false), Condvar::new())),
        }
    }

    pub fn send(&self, msg: T) {
        let &(ref sguard, ref scvar) = &*self.sleeping;
        let mut guard = sguard.lock().unwrap();
        let mut queue = self.queue.lock().unwrap();
        (*queue).push_back(msg);
        *guard = true;
        scvar.notify_one();
    }

    fn try_recv(&self) -> Result<T, MessageQueueError> {
        let mut queue = self.queue.lock().unwrap();
        match queue.pop_front() {
            Some(t) => Ok(t),
            None => Err(MessageQueueError::Empty),
        }
    }

    pub fn recv(&self) -> Result<T, MessageQueueError> {
        match self.try_recv() {
            Ok(head) => return Ok(head),
            Err(MessageQueueError::Empty) => (),
        }

        let ret;
        let &(ref sguard, ref scvar) = &*self.sleeping;
        let mut guard = sguard.lock().unwrap();
        loop {
            match self.try_recv() {
                Ok(t) => {
                    ret = Ok(t);
                    break;
                }
                Err(MessageQueueError::Empty) => (),
            }
            guard = scvar.wait(guard).unwrap();
        }
        ret
    }
}

impl<T> Clone for MessageQueue<T> {
    fn clone(&self) -> MessageQueue<T> {
        MessageQueue {
            queue: self.queue.clone(),
            sleeping: self.sleeping.clone(),
        }
    }
}
