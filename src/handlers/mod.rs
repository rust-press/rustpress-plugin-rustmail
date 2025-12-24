//! RustMail HTTP Handlers

pub mod email;
pub mod template;
pub mod queue;
pub mod log;

pub use email::EmailHandler;
pub use template::TemplateHandler;
pub use queue::QueueHandler;
pub use log::LogHandler;
