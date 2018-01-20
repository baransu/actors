extern crate uuid;

use uuid::Uuid;
use std::sync::{Arc, Mutex};
use std::thread;
use std::any::Any;

use actors::context::Context;
use actors::actor_ref::{InnerMessage, Envelope};
use actors::message_queue::MessageQueue;
use actors::{Actor, Message, ActorRef};
use actors::root_actor::RootActor;

#[derive(Debug)]
enum SystemMessage {
    Died(usize),
}

enum ThreadMessage {
    Die(usize),
    Actor(ActorRef),
}

pub struct ActorSystem {
    global_queue: MessageQueue<SystemMessage>,
    threads_queue: MessageQueue<ThreadMessage>,
    alive_threads: Arc<Mutex<usize>>,
    root: ActorRef,
}

impl ActorSystem {
    pub fn new(num_of_threads: usize) -> ActorSystem {
        let global_queue: MessageQueue<SystemMessage> = MessageQueue::new();
        let threads_queue: MessageQueue<ThreadMessage> = MessageQueue::new();
        let alive_threads = Arc::new(Mutex::new(num_of_threads));

        // Here we're creating root actor which is a grandparent of all actors
        let root = {
            let pid = Uuid::new_v4();
            let actor_ref = ActorRef::new(pid, RootActor::new());
            actor_ref
        };

        let system = ActorSystem {
            global_queue,
            threads_queue,
            alive_threads,
            root,
        };

        for thread_id in 0..num_of_threads {
            let system = system.clone();
            thread::spawn(move || {
                loop {
                    let msg = {
                        let head = system.threads_queue.recv();
                        if let Ok(msg) = head { msg } else { continue }
                    };

                    match msg {
                        ThreadMessage::Die(id) => {
                            if thread_id == id {
                                system.global_queue.send(SystemMessage::Died(thread_id));
                                println!("Thread {} terminated!", thread_id);
                                break;
                            } else {
                                // Message may be not for out thread so we have to forward it back to queue
                                system.threads_queue.send(msg);
                            }
                        }
                        ThreadMessage::Actor(actor_ref) => {
                            if let Some(envelope) = actor_ref.mailbox.lock().unwrap().pop_front() {
                                let msg = match envelope.message {
                                    InnerMessage::Message(msg) => msg,
                                };

                                let context = Context::new(
                                    system.clone(),
                                    actor_ref.clone(),
                                    envelope.sender,
                                );
                                actor_ref.inner.receive(msg, context);
                            }
                        }
                    }
                }
            });
        }
        system
    }

    pub fn terminate(&self) {
        // Push death order to all threads
        let alive = self.alive_threads.lock().unwrap();
        for id in 0..(*alive) {
            self.threads_queue.send(ThreadMessage::Die(id));
        }
        println!("Pushed death order to all {} threads.", *alive);
    }

    pub fn spawn(&self, actor: Arc<Actor>) -> ActorRef {
        self.spawn_of(self.root.clone(), actor)
    }

    pub fn spawn_of(&self, parent: ActorRef, actor: Arc<Actor>) -> ActorRef {
        let pid = Uuid::new_v4();
        let actor_ref = ActorRef::new(pid, actor);

        // We want to check if we have pid conflict or not
        let mut parent_children = parent.children.lock().unwrap();
        if !parent_children.contains_key(&pid) {
            parent_children.insert(pid, actor_ref.clone());
        } else {
            panic!("New actor pid duplication!");
        }
        actor_ref
    }

    pub fn tell<M: Message>(&self, sender: Option<ActorRef>, to: &ActorRef, message: M) {
        let message_to: Box<Any + Send> = Box::new(message);
        let envelope = Envelope {
            message: InnerMessage::Message(message_to),
            sender,
        };
        let mut mailbox = to.mailbox.lock().unwrap();
        (*mailbox).push_back(envelope);
        self.threads_queue.send(ThreadMessage::Actor(to.clone()));
    }

    pub fn run(&self) {
        // We're running as long as any thread is running
        loop {
            let head = self.global_queue.recv();
            match head {
                Err(_) => (),
                Ok(msg) => {
                    match msg {
                        SystemMessage::Died(_) => {
                            let mut alive = self.alive_threads.lock().unwrap();
                            (*alive) -= 1;
                            if *alive == 0 {
                                println!("All threads terminated. Stopping the system.");
                                break;
                            }
                        }
                    }
                }
            }
        }
    }
}

impl Clone for ActorSystem {
    fn clone(&self) -> ActorSystem {
        ActorSystem {
            global_queue: self.global_queue.clone(),
            threads_queue: self.threads_queue.clone(),
            alive_threads: self.alive_threads.clone(),
            root: self.root.clone(),
        }
    }
}
