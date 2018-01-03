extern crate uuid;

use uuid::Uuid;
use std::collections::HashMap;
use std::sync::{Mutex, Arc};
use std::thread;
use std::any::Any;

use actors::context::Context;
use actors::actor_ref::{InnerMessage, Envelope};
use actors::message_queue::MessageQueue;
use actors::{Actor, Message, ActorRef};

#[derive(Debug)]
enum SystemMessage {
    Died(usize),
}

enum ThreadMessage {
    // Die,
    Actor(ActorRef),
}

pub struct ActorSystem {
    global_queue: MessageQueue<SystemMessage>,
    actors_map: Arc<Mutex<HashMap<Uuid, ActorRef>>>,
    threads_queue: MessageQueue<ThreadMessage>,
    alive_threads: usize,
}

impl ActorSystem {
    pub fn new(num_of_threads: usize) -> ActorSystem {
        let global_queue: MessageQueue<SystemMessage> = MessageQueue::new();
        let threads_queue: MessageQueue<ThreadMessage> = MessageQueue::new();
        let actors_map: Arc<Mutex<HashMap<Uuid, ActorRef>>> = Arc::new(Mutex::new(HashMap::new()));
        let mut alive_threads = 0;

        let mut system = ActorSystem {
            global_queue,
            actors_map,
            threads_queue,
            alive_threads,
        };

        for _ in 0..num_of_threads {
            let system = system.clone();
            thread::spawn(move || {
                loop {
                    let msg = {
                        let head = system.threads_queue.recv();
                        match head {
                            Err(_) => continue,
                            Ok(msg) => msg,
                        }
                    };

                    match msg {
                        // ThreadMessage::Die => {
                        //     println!("Thread: {} is dying", thread_id);
                        //     system.global_queue.send(SystemMessage::Died(thread_id));
                        //     break;
                        // }
                        ThreadMessage::Actor(actor_ref) => {
                            // let entry =
                            // {
                            //     let actors = actors_map.lock().unwrap();
                            //     match actors.entry(actor_ref.pid) {
                            //         // there is not longer actor with this pid, we should skip the message then
                            //         Vacant(_) => continue,
                            //         Occupied(entry) => entry,
                            //     }
                            // };
                            // let a: &mut ActorRef = entry.into_mut() as &mut ActorRef;

                            match actor_ref.mailbox.lock().unwrap().pop_front() {
                                None => continue,
                                Some(envelope) => {
                                    let msg = match envelope.message {
                                        InnerMessage::Message(msg) => msg,
                                    };

                                    let context = Context::new(system.clone(), actor_ref.clone());
                                    let to_send = actor_ref.inner.receive(msg, context);
                                    match to_send {
                                        None => (),
                                        Some(messages) => {
                                            for msg in messages {
                                                // create messages and put them into ThreadsQueue
                                                println!(
                                                    "Actor receive returned message {:?}",
                                                    msg
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            });
            alive_threads += 1;
        }
        system.alive_threads = alive_threads;
        system
    }

    // TODO: shutdown that allows system actor the shutdown everything when there no more work
    // fn shutdown(&self) {}

    // TODO: system should have something like system actor which is parent of all actors
    // when system actor terminates we can kill all actors when because we're storing pids of childs
    pub fn spawn(&self, actor: Arc<Actor>) -> ActorRef {
        let pid = Uuid::new_v4();
        let actor_ref = ActorRef::new(pid, actor);

        // check if we have pids conflict.
        let mut actors = self.actors_map.lock().unwrap();
        if !actors.contains_key(&pid) {
            actors.insert(pid, actor_ref.clone());
        } else {
            panic!("We have pid duplication!!!");
        }
        // TODO: add pid to parent children list
        actor_ref
    }

    pub fn tell<M: Message>(&self, to: &ActorRef, message: M) {
        let message_to: Box<Any + Send> = Box::new(message);
        let envelope = Envelope { message: InnerMessage::Message(message_to) };
        let mut mailbox = to.mailbox.lock().unwrap();
        (*mailbox).push_back(envelope);
        self.threads_queue.send(ThreadMessage::Actor(to.clone()));
    }

    pub fn is_alive(&self) -> bool {
        self.alive_threads > 0
    }
}

impl Clone for ActorSystem {
    fn clone(&self) -> ActorSystem {
        ActorSystem {
            global_queue: self.global_queue.clone(),
            actors_map: self.actors_map.clone(),
            threads_queue: self.threads_queue.clone(),
            alive_threads: self.alive_threads,
        }
    }
}
