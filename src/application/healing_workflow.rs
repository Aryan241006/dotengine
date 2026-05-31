use crate::domain::{ConfigFile, ErrorPayload};
use crate::ports::{AiService, SystemManager};
use crate::ui::{accent, activity, heading};
use std::sync::Arc;

#[derive(Clone)]
pub struct HealingWorkflow {
    ai_service: Arc<dyn AiService>,
    system_manager: Arc<dyn SystemManager>,
    max_retries: usize,
}

impl HealingWorkflow {
    pub fn new(
        ai_service: Arc<dyn AiService>,
        system_manager: Arc<dyn SystemManager>,
        max_retries: usize,
    ) -> Self {
        Self {
            ai_service,
            system_manager,
            max_retries,
        }
    }

    /// Recursively heals system configurations by capturing logs, requesting AI fixes, and verifying
    pub async fn execute(
        &self,
        error_payload: ErrorPayload,
        design_rules_content: &str,
        attempt: usize,
    ) -> Result<Vec<ConfigFile>, Box<dyn std::error::Error + Send + Sync>> {
        if self.max_retries != usize::MAX && attempt > self.max_retries {
            println!(
                "{} Maximum self-healing attempts ({}) reached. Aborting.",
                accent("Dotengine"),
                self.max_retries
            );
            return Err(format!(
                "Failed to auto-heal Hyprland configurations after {} attempts.",
                self.max_retries
            )
            .into());
        }

        println!("\n{}", heading("Dotengine self-healing recovery"));
        if self.max_retries == usize::MAX {
            println!("Attempt {} (unlimited)", attempt);
        } else {
            println!("Attempt {} of {}", attempt, self.max_retries);
        }
        let repaired_configs = match activity(
            "Analyzing reload diagnostics",
            self.ai_service
                .repair_config(&error_payload, design_rules_content),
        )
        .await
        {
            Ok(configs) => configs,
            Err(e) => {
                println!(
                    "{} Failed to receive corrected configs from AI: {}",
                    accent("Dotengine"),
                    e
                );
                return Err(e);
            }
        };

        println!("{} Received repaired patches.", accent("Dotengine"));
        if !self
            .system_manager
            .confirm_config_changes(&repaired_configs)
            .await?
        {
            return Err("User declined repaired configuration changes".into());
        }

        println!(
            "{} Applying approved repaired patches...",
            accent("Dotengine")
        );
        for config in &repaired_configs {
            println!("    - Re-applying: '~/{}'", config.relative_path.display());
            self.system_manager.write_config_file(config).await?;
        }

        match activity(
            "Verifying corrected configuration",
            self.system_manager.verify_and_reload(&repaired_configs, None),
        )
        .await
        {
            Ok(_) => {
                println!(
                    "\n{} System recovered and applied corrected configurations.",
                    accent("Dotengine")
                );
                println!();
                Ok(repaired_configs)
            }
            Err(new_error_payload) => {
                println!(
                    "\n{} Repaired configs still have syntax or reload faults.",
                    accent("Dotengine")
                );
                println!("    Faulting command: {}", new_error_payload.command);
                println!("    Stderr error:\n    {}", new_error_payload.stderr);

                // Recurse / retry
                Box::pin(self.execute(new_error_payload, design_rules_content, attempt + 1)).await
            }
        }
    }
}
