pub mod cache;
pub mod clone;
pub mod source;

pub use cache::{clear_cache, get_or_clone, list_cached, CacheMetadata, CachedTemplate};
pub use clone::{clone_template, CloneResult};
pub use source::{
    format_resolved_source, resolve_git_protocol, resolve_source, GitProtocol, ResolveOptions,
    TemplateSource,
};
