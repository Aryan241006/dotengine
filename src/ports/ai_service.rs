use crate::domain::{ConfigFile, DesignReferenceSpec, ErrorPayload, SystemContext, UserPrompt};
use async_trait::async_trait;

#[async_trait]
pub trait AiService: Send + Sync {
    /// Extracts a structured design manifest from a reference screenshot or image set.
    async fn analyze_design_reference(
        &self,
        prompt: &UserPrompt,
        system_context: &SystemContext,
        design_rules: &str,
    ) -> Result<DesignReferenceSpec, Box<dyn std::error::Error + Send + Sync>>;

    /// Generates config files from natural language prompt, system properties, and visual design rules
    async fn generate_config(
        &self,
        prompt: &UserPrompt,
        system_context: &SystemContext,
        design_rules: &str,
    ) -> Result<Vec<ConfigFile>, Box<dyn std::error::Error + Send + Sync>>;

    /// Requests the AI to heal the configuration by analysing the runtime error diagnostics and logs
    async fn repair_config(
        &self,
        error_payload: &ErrorPayload,
        design_rules: &str,
    ) -> Result<Vec<ConfigFile>, Box<dyn std::error::Error + Send + Sync>>;
}
