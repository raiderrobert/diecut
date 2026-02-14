pub mod context;
pub mod file;
pub mod walker;

pub use context::{build_context, build_context_with_namespace};
pub use walker::{
    execute_plan, plan_render, walk_and_render, GeneratedProject, GenerationPlan, PlannedFile,
};
