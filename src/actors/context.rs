use actors::{ActorSystem, ActorRef};

pub struct Context {
    pub system: ActorSystem,
    pub sender: ActorRef,
}

impl Context {
    pub fn new(system: ActorSystem, sender: ActorRef) -> Context {
        Context { system, sender }
    }
}
