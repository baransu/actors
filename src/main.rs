extern crate rand;

use rand::Rng;
use std::collections::VecDeque;
use std::sync::{Mutex, Arc};
use std::thread;
use std::time::Duration;

const NUM_OF_THREADS: u32 = 8;
const LIFETIME: u32 = 1000;

enum Actor {
    SendMessage { message: f32, legs: usize },
    Dog { weight: f32, legs: usize },
    Monkey {
        weight: f32,
        arms: usize,
        legs: usize,
    },
    Fish { weight: f32, fins: usize },
    Dolphin { weight: f32, fins: usize },
    Snake { weight: f32, fangs: usize },
}


// there should top level application thread that is running always
fn actor(message: u32, state: &u32) -> (u32, Vec<Messages>) {
    println!("Updating actor state by {}!", message);
    (state + message, vec![])
}


enum Messages {
    Heartbeat { thread_id: u32, payload: u32 },
    Stop { thread_id: u32 },
}

fn main() {
    let queue: Arc<Mutex<VecDeque<Messages>>> = Arc::new(Mutex::new(VecDeque::new()));
    let actors: Arc<Mutex<Vec<u32>>> = Arc::new(Mutex::new(Vec::new()));
    let mut threads = vec![];

    for thread_id in 0..NUM_OF_THREADS {
        {
            actors.lock().unwrap().push(0);
        }
        let queue = Arc::clone(&queue);
        let actors = Arc::clone(&actors);
        let thread = thread::spawn(move || {
            let mut rng = rand::thread_rng();
            // let ms: u64 = rng.gen_range(0, 100);
            let rand_millis = Duration::from_millis(10);
            let mut i = 0;

            loop {
                {
                    let mut locked_actors = actors.lock().unwrap();
                    unsafe {
                        let mut a = locked_actors.get_unchecked_mut(thread_id as usize);
                        let (state, messages) = actor(thread_id, a);
                        *a = state;
                    }
                }

                let mut q = queue.lock().unwrap(); // scoped lock

                q.push_back(Messages::Heartbeat {
                    thread_id: thread_id,
                    payload: i,
                });
                i = i + 1;
                if i > LIFETIME {
                    q.push_back(Messages::Stop { thread_id: thread_id });
                    break;
                }
            }
        });
        threads.push(thread);
    }

    let mut i = 0;
    loop {
        match queue.lock().unwrap().pop_front() {
            Some(msg) => {
                match msg {
                    Messages::Heartbeat { thread_id, payload } => {
                        println!("Message from {} is: {}", thread_id, payload);
                    }
                    Messages::Stop { thread_id } => {
                        println!("The winner is {}!", thread_id);
                        break;
                    }
                    _ => (),
                }
            }
            None => (),
        }
    }

    for thread in threads {
        thread.join().unwrap();
    }

    for a in actors.lock().unwrap().iter() {
        println!("Actor state is: {}", a)
    }
}
