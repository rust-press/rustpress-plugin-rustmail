//! RustMail Services

pub mod mailer;
pub mod template;
pub mod queue;
pub mod log;
pub mod smtp;

pub use mailer::MailerService;
pub use template::TemplateService;
pub use queue::QueueService;
pub use log::LogService;
pub use smtp::SmtpTransport;
