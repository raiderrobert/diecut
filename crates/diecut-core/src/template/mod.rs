pub mod cache;
pub mod clone;
pub mod source;

pub use cache::{get_or_clone, list_cached, clear_cache, CachedTemplate};
pub use clone::clone_template;
pub use source::{resolve_source, resolve_source_with_ref, TemplateSource};
