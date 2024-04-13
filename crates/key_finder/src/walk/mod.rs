// mod file_walk;
mod dom_walker;
mod website_walk;
pub mod error;

pub(self) use dom_walker::{DomVisitor, DomWalker};
pub use website_walk::{ScriptMessage, ScriptReceiver, ScriptSender, WebsiteWalker};
