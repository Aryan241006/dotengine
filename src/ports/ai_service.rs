use crate::domain::{ConfigFile, ErrorPayload, SystemContext, UserPrompt};
use async_trait::async_trait;

#[async_trait]
pub trait AiService: Send + Sync {
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
