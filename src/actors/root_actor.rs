use actors::context::Context;
use std::any::Any;
use actors::Actor;
use std::sync::Arc;

pub struct RootActor;

impl Actor for RootActor {
    fn new() -> Arc<RootActor> {
        Arc::new(RootActor)
    }

    fn receive(&self, _message: Box<Any>, _context: Context) {}
}
