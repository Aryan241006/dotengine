pub mod cli_plan;
pub mod component_normalization;
pub mod design_archetypes;
pub mod desktop_audit;
pub mod generation_workflow;
pub mod healing_workflow;
pub mod skill_corpus;

pub use cli_plan::{BackupMode, ComponentStack, ExistingStack, PromptIntent, RunPlan};
pub use component_normalization::{normalize_component_value, normalize_stack_values};
pub use design_archetypes::{
    archetype_guidelines, archetype_recommendation_lines, infer_archetype,
};
pub use desktop_audit::{audit_desktop_completeness, DesktopCompletenessReport};
pub use generation_workflow::GenerationWorkflow;
pub use healing_workflow::HealingWorkflow;
