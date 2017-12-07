extern crate rand;
extern crate uuid;

// use std::any::Any;
// use rand::Rng;
use uuid::Uuid;
use std::collections::HashMap;
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::VecDeque;
use std::sync::{Mutex, Arc};
use std::thread;
// use std::time::Duration;

const NUM_OF_THREADS: usize = 8;
const LIFETIME: u32 = 1000;

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
    fn receive(message: Message, state: u32) -> (u32, Option<Vec<Message>>) {
        // example actor implementation
        // in the future will be trait that every actor will have to implement
        (state, None)
    }
}

type GlobalQueue = Arc<Mutex<VecDeque<SystemMessage>>>;
type ActorsQueue = Arc<Mutex<VecDeque<Uuid>>>;
type ActorsMap = Arc<Mutex<HashMap<Uuid, Actor>>>;
type ThreadsQueue = Arc<Mutex<VecDeque<SystemMessage>>>;

struct ActorSystem {
    global_queue: GlobalQueue,
    actors_queue: ActorsQueue,
    actors_map: ActorsMap,
    threads_queue: ThreadsQueue,
    alive_threads: usize,
}

impl ActorSystem {
    fn new() -> ActorSystem {
        let global_queue: GlobalQueue = Arc::new(Mutex::new(VecDeque::new()));
        let actors_queue: ActorsQueue = Arc::new(Mutex::new(VecDeque::new()));
        let threads_queue: ThreadsQueue = Arc::new(Mutex::new(VecDeque::new()));
        let actors_map: ActorsMap = Arc::new(Mutex::new(HashMap::new()));
        let mut alive_threads = 0;

        for thread_id in 0..NUM_OF_THREADS {
            let global_queue = Arc::clone(&global_queue);
            let actors_queue = Arc::clone(&actors_queue);
            let actors_map = Arc::clone(&actors_map);
            let threads_queue = Arc::clone(&threads_queue);
            let _ = thread::spawn(move || {
                loop {
                    let should_i_die = {
                        match threads_queue.lock().unwrap().front() {
                            None => false,
                            Some(msg) => {
                                match *msg {
                                    // thread received order to die
                                    SystemMessage::Die(id) => id == thread_id,
                                    _ => false,
                                }
                            }
                        }
                    };

                    if should_i_die {
                        threads_queue.lock().unwrap().pop_front();
                        println!("Thread: {} is dying", thread_id);
                        global_queue.lock().unwrap().push_back(
                            SystemMessage::Died(thread_id),
                        );
                        break;
                    }

                    println!("Thread {} is running", thread_id);
                    thread::sleep_ms(100);
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
                                    let mut a: &mut Actor = entry.into_mut() as &mut Actor;
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
            global_queue: global_queue,
            actors_queue: actors_queue,
            actors_map: actors_map,
            threads_queue: threads_queue,
            alive_threads: alive_threads,
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
enum SystemMessage {
    Die(usize),
    Died(usize),
}

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

    thread::sleep_ms(5000);
    for thread_id in 0..NUM_OF_THREADS {
        system.threads_queue.lock().unwrap().push_back(
            SystemMessage::Die(
                thread_id,
            ),
        );
        println!("Pushed death order")
    }

    loop {
        if system.alive_threads == 0 {
            break;
        }
        match system.global_queue.lock().unwrap().pop_front() {
            None => continue,
            Some(msg) => {
                match msg {
                    SystemMessage::Died(id) => {
                        println!("Thread {} died!", id);
                        system.alive_threads -= 1;
                    }
                    _ => continue,
                }
            }
        }
    }


    // for thread in threads {
    //     thread.join().unwrap();
    // }

    // for a in actors.lock().unwrap().iter() {
    //     println!("Actor state is: {}", a)
    // }
}
