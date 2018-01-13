extern crate uuid;

use std::collections::{VecDeque, HashMap};
use uuid::Uuid;
use std::sync::{Mutex, Arc};
use std::any::Any;

use actors::Actor;

pub struct ActorRef {
    pub pid: Uuid,
    pub inner: Arc<Actor>,
    pub mailbox: Arc<Mutex<VecDeque<Envelope>>>,
    pub children: Arc<Mutex<HashMap<Uuid, ActorRef>>>,
}

pub struct Envelope {
    pub message: InnerMessage,
    pub sender: Option<ActorRef>,
}

pub enum InnerMessage {
    Message(Box<Any + Send>),
}

// Actor info and struct
impl ActorRef {
    pub fn new(pid: Uuid, actor: Arc<Actor>) -> ActorRef {
        ActorRef {
            pid,
            inner: actor.clone(),
            mailbox: Arc::new(Mutex::new(VecDeque::new())),
            children: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl Clone for ActorRef {
    fn clone(&self) -> ActorRef {
        ActorRef {
            pid: self.pid,
            inner: self.inner.clone(),
            mailbox: self.mailbox.clone(),
            children: self.children.clone(),
        }
    }
}
