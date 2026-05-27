use super::config_file::ConfigFile;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ErrorPayload {
    /// The specific command that generated the error (e.g. `hyprctl reload`)
    pub command: String,

    /// The exit status code of the failed execution command
    pub exit_code: Option<i32>,

    /// Diagnostic standard output captured during the execution
    pub stdout: String,

    /// Diagnostic standard error captured during execution, containing syntax/compilation faults
    pub stderr: String,

    /// The set of generated configuration files which led to the validation failure
    pub active_configs: Vec<ConfigFile>,
}

impl ErrorPayload {
    pub fn new(
        command: String,
        exit_code: Option<i32>,
        stdout: String,
        stderr: String,
        active_configs: Vec<ConfigFile>,
    ) -> Self {
        Self {
            command,
            exit_code,
            stdout,
            stderr,
            active_configs,
        }
    }
}
