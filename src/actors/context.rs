use actors::{ActorSystem, ActorRef};

pub struct Context {
    pub system: ActorSystem,
    pub sender: Option<ActorRef>,
    pub me: ActorRef,
}

impl Context {
    pub fn new(system: ActorSystem, me: ActorRef, sender: Option<ActorRef>) -> Context {
        Context { system, sender, me }
    }
}
