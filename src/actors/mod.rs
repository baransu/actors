use std::any::Any;
use std::sync::Arc;

pub mod message_queue;
pub mod root_actor;

pub mod context;
pub use self::context::Context;

pub mod actor_ref;
pub use self::actor_ref::ActorRef;

pub mod actor_system;
pub use self::actor_system::ActorSystem;

// Message trait
pub trait Message: Clone + Send + Sync + 'static + Any {}
impl<T> Message for T
where
    T: Clone + Send + Sync + 'static + Any,
{
}

// Trait that every actors has to implement to be an actor
pub trait Actor: Send + Sync + 'static {
    fn new() -> Arc<Self>
    where
        Self: Sized;

    fn receive(&self, message: Box<Any>, context: Context);
}
