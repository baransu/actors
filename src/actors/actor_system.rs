extern crate uuid;

use uuid::Uuid;
// use std::collections::HashMap;
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
    // actors_map: Arc<Mutex<HashMap<Uuid, ActorRef>>>,
    threads_queue: MessageQueue<ThreadMessage>,
    alive_threads: Arc<Mutex<usize>>,
    root: ActorRef,
}

impl ActorSystem {
    pub fn new(num_of_threads: usize) -> ActorSystem {
        let global_queue: MessageQueue<SystemMessage> = MessageQueue::new();
        let threads_queue: MessageQueue<ThreadMessage> = MessageQueue::new();
        // let actors_map: Arc<Mutex<HashMap<Uuid, ActorRef>>> = Arc::new(Mutex::new(HashMap::new()));
        let alive_threads = Arc::new(Mutex::new(num_of_threads));

        // Here we're creating root actor which a grandparent of all actors
        let root = {
            let pid = Uuid::new_v4();
            let actor_ref = ActorRef::new(pid, RootActor::new());
            actor_ref
        };

        let system = ActorSystem {
            global_queue,
            // actors_map,
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
                                println!("Thread: {} died", thread_id);
                                break;
                            } else {
                                // It's not for us so we have to forward it
                                system.threads_queue.send(msg);
                            }
                        }
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

                            if let Some(envelope) = actor_ref.mailbox.lock().unwrap().pop_front() {
                                let msg = match envelope.message {
                                    InnerMessage::Message(msg) => msg,
                                };

                                let context = Context::new(system.clone(), actor_ref.clone());
                                let to_send = actor_ref.inner.receive(msg, context);
                                if let Some(messages) = to_send {
                                    for msg in messages {
                                        // create messages and put them into ThreadsQueue
                                        println!("Actor returned message {:?}", msg);
                                    }
                                }
                            }
                        }
                    }
                }
            });
        }
        system
    }

    // Shutdown allows to stop all threads and clear all memory when there is no more work
    pub fn terminate(&self) {
        // Push death order to all threads
        let alive = self.alive_threads.lock().unwrap();
        for id in 0..(*alive) {
            self.threads_queue.send(ThreadMessage::Die(id));
            println!("Pushed death order")
        }
    }

    pub fn spawn(&self, actor: Arc<Actor>) -> ActorRef {
        self.spawn_of(self.root.clone(), actor)
    }

    pub fn spawn_of(&self, parent: ActorRef, actor: Arc<Actor>) -> ActorRef {
        let pid = Uuid::new_v4();
        let actor_ref = ActorRef::new(pid, actor);

        // check if we have pids conflict.
        let mut parent_children = parent.children.lock().unwrap();
        if !parent_children.contains_key(&pid) {
            parent_children.insert(pid, actor_ref.clone());
        } else {
            panic!("We have pid duplication!!!");
        }
        actor_ref
    }

    pub fn tell<M: Message>(&self, to: &ActorRef, message: M) {
        let message_to: Box<Any + Send> = Box::new(message);
        let envelope = Envelope { message: InnerMessage::Message(message_to) };
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
                        SystemMessage::Died(id) => {
                            let mut alive = self.alive_threads.lock().unwrap();
                            println!("Thread {} died!", id);
                            (*alive) -= 1;
                            println!("Alive threads: {}", *alive);
                            if *alive == 0 {
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
            // actors_map: self.actors_map.clone(),
            threads_queue: self.threads_queue.clone(),
            alive_threads: self.alive_threads.clone(),
            root: self.root.clone(),
        }
    }
}
