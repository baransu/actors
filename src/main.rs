extern crate uuid;

mod actors;

use std::sync::{Mutex, Arc};
use std::any::Any;
use std::fs::File;
use std::io::prelude::*;

use actors::{Actor, ActorSystem, Context};

#[derive(Clone, Debug)]
enum Message<'a> {
    ProcessFile(&'a str),
    ProcessLine(String),
    Result(i32),
}

struct ManagerState {
    count: i32,
    processing: i32,
}

impl ManagerState {
    fn new() -> ManagerState {
        ManagerState {
            count: 0,
            processing: 0,
        }
    }
}

struct Manager {
    state: Mutex<ManagerState>,
}

impl Actor for Manager {
    fn new() -> Arc<Manager> {
        let state = Mutex::new(ManagerState::new());
        Arc::new(Manager { state })
    }

    fn receive(&self, message: Box<Any>, context: Context) {
        let msg = message.downcast::<Message>().unwrap();
        match *msg {
            Message::ProcessFile(filename) => {
                // Read file
                let mut file = File::open(filename).unwrap();
                let mut file_content = String::new();
                file.read_to_string(&mut file_content).unwrap();

                // Split by lines
                let lines: Vec<&str> = file_content.lines().collect();
                let count = lines.len();

                // Set how many lines we have and how many results we're expecting
                let mut state = self.state.lock().unwrap();
                (*state).processing = count as i32;

                // Spawn workers and send workers to them
                for line in lines.iter() {
                    let msg = Message::ProcessLine(line.to_string());
                    let worker = context.system.spawn(Worker::new());
                    context.system.tell(Some(context.me.clone()), &worker, msg);
                }
            }
            Message::Result(count) => {
                let mut state = self.state.lock().unwrap();
                (*state).processing -= 1;
                (*state).count += count;

                println!(
                    "Received result {} so all words count is {}.",
                    count,
                    (*state).count
                );

                if (*state).processing == 0 {
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
                let characters: Vec<&str> = line.split(' ').collect();

                // If we have sender we can send back the result
                if let Some(sender) = context.sender {
                    context.system.tell(
                        None,
                        &sender,
                        Message::Result(characters.len() as i32),
                    )
                }
            }
            _ => (),
        }
    }
}


fn main() {
    let system = ActorSystem::new(8);
    let manager = system.spawn(Manager::new());
    system.tell(None, &manager, Message::ProcessFile("foo.txt"));

    // Blocks main thread as long as system is running
    system.run();
}
