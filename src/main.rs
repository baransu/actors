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


fn should_i_die(front: &Option<&SystemMessage>) -> bool {
    match *front {
        None => false,
        Some(msg) => {
            match *msg {
                // thread received order to die
                SystemMessage::Die => true,
                _ => false,
            }
        }
    }
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
}

// unsafe impl<T: Copy> Send for Sender<T> { }

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

                // threads queue - spmc (supervisor to threads)
                // let &(ref tlock, ref tcvar) = &*threads_queue.sleeping;

                // global queue - mpsc (threads to supervisor)
                let &(ref glock, ref gcvar) = &*global_queue.sleeping;

                loop {
                    // let mut new_messages = tlock.lock().unwrap();
                    // let result = tcvar
                    //     .wait_timeout(new_messages, Duration::from_millis(10))
                    //     .unwrap();
                    // // 10 milliseconds have passed, or maybe the value changed!
                    // new_messages = result.0;
                    // let queue = threads_queue.queue.lock().unwrap();
                    // let head = queue.front();
                    // println!("New messages, {}, {:?}", *new_messages, head);
                    // if !*new_messages {
                    //     continue;
                    // }
                    let im_dying = true;
                    // {
                    //     let mut queue = threads_queue.queue.lock().unwrap();
                    //     let head = queue.front();
                    //     should_i_die(&head)
                    // };

                    println!("Should I die: {}", im_dying);
                    thread::sleep_ms(100);
                    if im_dying {
                        let mut g_changes = glock.lock().unwrap();
                        println!("Thread: {} is dying", thread_id);
                        let mut queue = global_queue.queue.lock().unwrap();
                        (*queue).push_back(SystemMessage::Died(thread_id));
                        *g_changes = true;
                        gcvar.notify_one();
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

                    // q.push_back(Messages::Heartbeat {
                    //     thread_id: thread_id,
                    //     payload: i,
                    // });
                    // i = i + 1;
                    // if i > LIFETIME {
                    //     q.push_back(Messages::Stop { thread_id: thread_id });
                    //     break;
                    // }
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

// TODO: concider producer - consumer design
// https://www.reddit.com/r/rust/comments/5w7fwo/whats_the_best_way_to_do_producer_consumer_in_rust/
// https://github.com/seanmonstar/spmc

fn main() {
    let mut system = ActorSystem::new();
    // let mut i = 0;
    // loop {
    //     match queue.lock().unwrap().pop_front() {
    //         Some(msg) => {
    //             match msg {
    //                 Messages::Heartbeat { thread_id, payload } => {
    //                     println!("Message from {} is: {}", thread_id, payload);
    //                 }
    //                 Messages::Stop { thread_id } => {
    //                     println!("The winner is {}!", thread_id);
    //                     break;
    //                 }
    //                 _ => (),
    //             }
    //         }
    //         None => (),
    //     }
    // }

    let &(ref tlock, ref tcvar) = &*system.threads_queue.sleeping;

    let mut queue = system.threads_queue.queue.lock().unwrap();
    for _ in 0..NUM_OF_THREADS {
        (*queue).push_back(SystemMessage::Die);
        println!("Pushed death order")
    }

    // notify all about death order
    let mut t_changes = tlock.lock().unwrap();
    *t_changes = true;
    tcvar.notify_one();

    let &(ref glock, ref gcvar) = &*system.global_queue.sleeping;
    while system.alive_threads > 0 {
        let mut g_changes = glock.lock().unwrap();
        println!("Started waiting for changes");
        // TODO: try receive or wait
        g_changes = gcvar.wait(g_changes).unwrap();
        println!("Global changes: {}", g_changes);
        if !*g_changes {
            continue;
        }
        let mut queue = system.global_queue.queue.lock().unwrap();
        match (*queue).pop_front() {
            None => continue,
            Some(msg) => {
                match msg {
                    SystemMessage::Died(id) => {
                        println!("Thread {} died!", id);
                        system.alive_threads -= 1;
                        println!("Alive threads: {}", system.alive_threads);
                        let messages_left = (*queue).len();
                        println!("Messages left: {}", messages_left);
                        if messages_left > 0 {
                            continue;
                        }
                    }
                    _ => continue,
                }
            }
        }
    }
}
