extern crate rand;
extern crate uuid;

// use std::any::Any;
// use rand::Rng;
use uuid::Uuid;
use std::collections::HashMap;
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::VecDeque;
use std::sync::{Mutex, Arc, Condvar};
use std::thread;
use std::time::Duration;

const NUM_OF_THREADS: usize = 8;

// struct Actor { state + receive + mailbox }
// struct Thread { JoinHandle, message queue }
// enum SystemMessage
// enum ThreadMessage - meant for actor system -> thread communication

#[derive(Debug)]
enum Message {
    Increment(u32),
}

struct Actor {
    state: u32,
    pid: Uuid,
    mailbox: Arc<Mutex<VecDeque<Message>>>,
}

impl Actor {
    // TODO: we should be able to send system messages as well
    fn receive(_message: Message, state: u32) -> (u32, Option<Vec<Message>>) {
        // example actor implementation
        // in the future will be trait that every actor will have to implement
        (state, None)
    }
}

type Queue = MessageQueue<SystemMessage>;
type GlobalQueue = Queue;
type ActorsQueue = Arc<Mutex<VecDeque<Uuid>>>;
type ActorsMap = Arc<Mutex<HashMap<Uuid, Actor>>>;
type ThreadsQueue = Queue;

struct ActorSystem {
    global_queue: GlobalQueue,
    actors_queue: ActorsQueue,
    actors_map: ActorsMap,
    threads_queue: ThreadsQueue,
    alive_threads: usize,
}


fn should_i_die(msg: SystemMessage) -> bool {
    match msg {
        SystemMessage::Die => true,
        _ => false,
    }
}

#[derive(Debug)]
enum MessageQueueError {
    Empty,
}

struct MessageQueue<T> {
    queue: Arc<Mutex<VecDeque<T>>>,
    sleeping: Arc<(Mutex<bool>, Condvar)>,
}

impl<T> MessageQueue<T> {
    fn new() -> MessageQueue<T> {
        MessageQueue {
            queue: Arc::new(Mutex::new(VecDeque::new())),
            sleeping: Arc::new((Mutex::new(false), Condvar::new())),
        }
    }

    fn clone(&self) -> MessageQueue<T> {
        MessageQueue {
            queue: self.queue.clone(),
            sleeping: self.sleeping.clone(),
        }
    }

    fn send(&self, msg: T) {
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

    fn recv(&self) -> Result<T, MessageQueueError> {
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

impl ActorSystem {
    fn new() -> ActorSystem {
        let global_queue: Queue = MessageQueue::new();
        let actors_queue: ActorsQueue = Arc::new(Mutex::new(VecDeque::new()));
        let threads_queue: Queue = MessageQueue::new();
        let actors_map: ActorsMap = Arc::new(Mutex::new(HashMap::new()));
        let mut alive_threads = 0;

        for thread_id in 0..NUM_OF_THREADS {
            let global_queue = global_queue.clone();
            let actors_queue = actors_queue.clone();
            let actors_map = actors_map.clone();
            let threads_queue = threads_queue.clone();
            thread::spawn(move || {

                loop {
                    let im_dying = {
                        let head = threads_queue.recv();
                        match head {
                            Err(_) => continue,
                            Ok(msg) => should_i_die(msg),
                        }
                    };

                    println!("Should I die: {}", im_dying);

                    if im_dying {
                        println!("Thread: {} is dying", thread_id);
                        global_queue.send(SystemMessage::Died(thread_id));
                        break;
                    }

                    println!("Thread {} is running", thread_id);

                    let mut aq = actors_queue.lock().unwrap();
                    match aq.pop_front() {
                        None => continue,
                        Some(pid) => {
                            println!("We received some actor in queue");
                            // we have some actor, now we have get his body and process him
                            let mut actors = actors_map.lock().unwrap();
                            match actors.entry(pid) {
                                // there is not longer actor with this pid, we should skip the message
                                Vacant(_) => continue,
                                Occupied(entry) => {
                                    let a: &mut Actor = entry.into_mut() as &mut Actor;
                                    match a.mailbox.lock().unwrap().pop_front() {
                                        None => continue,
                                        Some(msg) => {
                                            let (new_state, to_send) = Actor::receive(msg, a.state);
                                            a.state = new_state;
                                            match to_send {
                                                None => (),
                                                Some(messages) => {
                                                    for msg in messages {
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
                    }
                }
            });
            alive_threads += 1;
        }

        ActorSystem {
            global_queue,
            actors_queue,
            actors_map,
            threads_queue,
            alive_threads,
        }
    }

    fn spawn_actor(&self, state: u32) -> Uuid {
        let pid = Uuid::new_v4();
        let actor = Actor {
            state: state,
            pid: pid,
            mailbox: Arc::new(Mutex::new(VecDeque::new())),
        };

        // check if we have pids conflict.
        let mut actors = self.actors_map.lock().unwrap();
        if !actors.contains_key(&pid) {
            actors.insert(pid, actor);
        } else {
            panic!("We have pid duplication!!!");
        }
        pid
    }
}

// SystemMessages are available in all actors and allow us to interact with actor system in the runtime
#[derive(Debug)]
enum SystemMessage {
    Die,
    Died(usize),
}

fn main() {
    let mut system = ActorSystem::new();

    // kill all threads
    for _ in 0..NUM_OF_THREADS {
        system.threads_queue.send(SystemMessage::Die);
        println!("Pushed death order")
    }

    // wait for all threads death
    while system.alive_threads > 0 {
        let head = system.global_queue.recv();
        match head {
            Ok(msg) => {
                match msg {
                    SystemMessage::Died(id) => {
                        println!("Thread {} died!", id);
                        system.alive_threads -= 1;
                        println!("Alive threads: {}", system.alive_threads);
                    }
                    _ => (),
                }
            }
            Err(_) => println!("Head is empty"),

        }
    }
}
