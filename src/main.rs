extern crate uuid;

mod actors;

use std::time::Instant;
use std::sync::{Mutex, Arc};
use std::any::Any;
use std::fs::File;
use std::io::prelude::*;

use actors::{Actor, ActorSystem, Context, ActorRef};

#[derive(Clone, Debug)]
enum Message<'a> {
    ProcessFile(&'a str, i32),
    ProcessLine(String),
    Result(i32),
}

struct ManagerState {
    workers: Vec<(bool, ActorRef)>,
    count: i32,
    to_process: Vec<String>,
}

impl ManagerState {
    fn new() -> ManagerState {
        ManagerState {
            workers: Vec::new(),
            count: 0,
            to_process: Vec::new(),
        }
    }
}

struct Manager {
    state: Mutex<ManagerState>,
}

impl Manager {
    fn send_to_available_workers(&self, context: &Context) {

        let mut state = self.state.lock().unwrap();
        let mut lines = (*state).to_process.clone();

        if (*state).to_process.len() > 0 {
            for w in (*state).workers.iter_mut() {
                if (*w).0 {
                    let line = lines.pop().unwrap();
                    let msg = Message::ProcessLine(line.to_string());
                    context.system.tell(
                        Some(context.me.clone()),
                        &(w.1).clone(),
                        msg,
                    );

                    (*w).0 = false;
                }
            }
            (*state).to_process = lines.clone();
        }
    }

    fn get_busy_count(&self) -> i32 {
        let state = self.state.lock().unwrap();
        let mut busy = 0;
        for worker in (*state).workers.iter() {
            if !(*worker).0 {
                busy += 1;
            }
        }
        busy
    }

    fn get_to_process_count(&self) -> i32 {
        let state = self.state.lock().unwrap();
        (*state).to_process.len() as i32
    }
}

impl Actor for Manager {
    fn new() -> Arc<Manager> {
        let state = Mutex::new(ManagerState::new());
        Arc::new(Manager { state })
    }

    fn receive(&self, message: Box<Any>, context: Context) {
        let msg = message.downcast::<Message>().unwrap();
        match *msg {
            Message::ProcessFile(filename, workers_count) => {
                // Read file
                let mut file = File::open(filename).unwrap();
                let mut file_content = String::new();
                file.read_to_string(&mut file_content).unwrap();

                // Split by lines
                let lines: Vec<&str> = file_content.lines().collect();

                // Set how many lines we have and how many results we're expecting
                {
                    let mut state = self.state.lock().unwrap();
                    (*state).to_process = lines.iter().map(|l| l.to_string()).collect();

                    // Create workers that we will reuse
                    println!("Spawning {} workers!", workers_count);
                    for _ in 0..workers_count {
                        let worker = context.system.spawn(Worker::new());
                        (*state).workers.push((true, worker));
                    }
                }

                // We're taking all free workser and sending work for them
                self.send_to_available_workers(&context)
            }
            Message::Result(count) => {
                {
                    let mut state = self.state.lock().unwrap();
                    (*state).count += count;
                    println!(
                        "Received result {} so all words count is {}.",
                        count,
                        (*state).count
                    );
                    if let Some(sender) = context.sender.clone() {
                        for worker in (*state).workers.iter_mut() {
                            if ActorRef::eq(sender.clone(), (*worker).1.clone()) {
                                (*worker).0 = true;
                            }
                        }
                    }
                }

                self.send_to_available_workers(&context);

                let busy = self.get_busy_count();
                let to_process = self.get_to_process_count();
                if to_process + busy == 0 {
                    context.system.terminate();
                }
            }
            _ => (),
        }
    }
}

struct Worker;

impl Actor for Worker {
    fn new() -> Arc<Worker> {
        Arc::new(Worker)
    }

    fn receive(&self, message: Box<Any>, context: Context) {
        let msg = message.downcast::<Message>().unwrap();
        match *msg {
            Message::ProcessLine(line) => {
                // Get words count
                let words: Vec<&str> = line.split(' ').collect();

                // If we have sender we can send back the result
                if let Some(sender) = context.sender {
                    context.system.tell(
                        Some(context.me),
                        &sender,
                        Message::Result(words.len() as i32),
                    )
                }
            }
            _ => (),
        }
    }
}


fn main() {
    let start = Instant::now();
    let system = ActorSystem::new(8);
    let manager = system.spawn(Manager::new());
    let workers = 10;
    let file = "foo.txt";
    system.tell(None, &manager, Message::ProcessFile(file, workers));

    // Blocks main thread as long as system is running
    system.run();

    let stop = start.elapsed();
    let elapsed_ms = (stop.as_secs() * 1_000) + (stop.subsec_nanos() / 1_000_000) as u64;

    println!(
        "Processed {} using {} workers in {} miliseconds",
        file,
        workers,
        elapsed_ms
    );
}
