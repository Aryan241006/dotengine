use crate::domain::{ConfigFile, ErrorPayload, SystemContext};
use async_trait::async_trait;
use std::path::{Path, PathBuf};

#[async_trait]
pub trait SystemManager: Send + Sync {
    /// Retrieves the safe root-scoped user home directory path
    fn get_home_directory(&self) -> PathBuf;

    /// Automatically scans and builds the current system profile and display settings
    async fn detect_system_context(
        &self,
    ) -> Result<SystemContext, Box<dyn std::error::Error + Send + Sync>>;

    /// Verifies if a CLI binary (e.g., `ags`, `rofi`) is available in the system $PATH
    async fn check_command_installed(&self, command: &str) -> bool;

    /// Prompts the user and runs appropriate command line commands to install missing applications
    async fn install_package(
        &self,
        package_name: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Presents a batch of supported config changes and obtains approval before applying them.
    async fn confirm_config_changes(
        &self,
        configs: &[ConfigFile],
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>>;

    /// Safely saves an approved configuration file, backing up existing content first.
    async fn write_config_file(
        &self,
        file: &ConfigFile,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Safely reads the content of an existing configuration file
    async fn read_config_file(
        &self,
        relative_path: &Path,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>>;

    /// Executes validation or reload commands (e.g., `hyprctl reload`), returning diagnostics upon failure
    async fn verify_and_reload(&self, configs: &[ConfigFile]) -> Result<(), ErrorPayload>;
}
