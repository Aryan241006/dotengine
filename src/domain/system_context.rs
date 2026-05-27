use std::collections::HashMap;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MonitorInfo {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub refresh_rate: f32,
    pub scale: f32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SystemContext {
    /// Host operating system or distribution identifier
    pub distribution: String,

    /// List of active displays connected to the machine
    pub monitors: Vec<MonitorInfo>,

    /// Map of package names and whether they are installed on the system (e.g. "rofi" -> true)
    pub package_status: HashMap<String, bool>,

    /// The native package manager available for the detected distribution (e.g. "pacman", "apt")
    pub package_manager: Option<String>,
}

impl SystemContext {
    pub fn new(distribution: String, package_manager: Option<String>) -> Self {
        Self {
            distribution,
            monitors: Vec::new(),
            package_status: HashMap::new(),
            package_manager,
        }
    }

    pub fn with_monitor(mut self, monitor: MonitorInfo) -> Self {
        self.monitors.push(monitor);
        self
    }

    pub fn with_packages(mut self, status: HashMap<String, bool>) -> Self {
        self.package_status = status;
        self
    }
}
