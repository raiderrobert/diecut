pub mod clone;
pub mod source;

pub use clone::clone_template;
pub use source::{resolve_source, resolve_source_with_ref, TemplateSource};
