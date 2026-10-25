pub mod admin;
pub mod consumer;
pub mod producer;

use std::fmt::Debug;

// crating the master trait
use admin::QueueAdmin;
use consumer::Consumer;
use producer::Producer;

/// A complete queue backend supporting publishing, consuming,
/// acknowledgements, and administrative operations.
pub trait Queue: Producer + Consumer + QueueAdmin + Debug {}

/// Automatically implement Queue for every type that implements
/// all three required traits.
impl<T> Queue for T where T: Producer + Consumer + QueueAdmin + Debug {}
