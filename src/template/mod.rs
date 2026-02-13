pub mod cache;
pub mod clone;
pub mod source;

pub use cache::{clear_cache, get_or_clone, list_cached, CacheMetadata, CachedTemplate};
pub use clone::{clone_template, CloneResult};
pub use source::{resolve_source, resolve_source_full, resolve_source_with_ref, TemplateSource};
