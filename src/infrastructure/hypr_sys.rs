use crate::domain::{ConfigFile, ErrorPayload, MonitorInfo, SystemContext};
use crate::ports::SystemManager;
use crate::ui::{accent, heading};
use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use std::process::Stdio;
use tokio::fs::{copy, create_dir_all, metadata, read_to_string, write};
use tokio::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
struct LinuxDistribution {
    display_name: String,
    id: String,
    id_like: Vec<String>,
}

impl LinuxDistribution {
    fn unknown() -> Self {
        Self {
            display_name: "Unknown Linux".to_string(),
            id: String::new(),
            id_like: Vec::new(),
        }
    }

    fn from_os_release(contents: &str) -> Self {
        let values: HashMap<&str, String> = contents
            .lines()
            .filter_map(|line| {
                let (key, value) = line.split_once('=')?;
                Some((
                    key,
                    value
                        .trim()
                        .trim_matches(|quote| quote == '"' || quote == '\'')
                        .to_string(),
                ))
            })
            .collect();

        let id = values.get("ID").cloned().unwrap_or_default().to_lowercase();
        let id_like = values
            .get("ID_LIKE")
            .map(|value| {
                value
                    .split_whitespace()
                    .map(|family| family.to_lowercase())
                    .collect()
            })
            .unwrap_or_default();
        let display_name = values
            .get("PRETTY_NAME")
            .or_else(|| values.get("NAME"))
            .cloned()
            .unwrap_or_else(|| "Unknown Linux".to_string());

        Self {
            display_name,
            id,
            id_like,
        }
    }

    fn belongs_to(&self, families: &[&str]) -> bool {
        families
            .iter()
            .any(|family| self.id == *family || self.id_like.iter().any(|id| id == family))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PackageManager {
    Apt,
    Paru,
    Yay,
    Pacman,
    Dnf,
    Yum,
    Zypper,
    Apk,
    Xbps,
    Emerge,
}

impl PackageManager {
    fn name(self) -> &'static str {
        match self {
            Self::Apt => "apt",
            Self::Paru => "paru",
            Self::Yay => "yay",
            Self::Pacman => "pacman",
            Self::Dnf => "dnf",
            Self::Yum => "yum",
            Self::Zypper => "zypper",
            Self::Apk => "apk",
            Self::Xbps => "xbps-install",
            Self::Emerge => "emerge",
        }
    }

    fn probe_binary(self) -> &'static str {
        match self {
            Self::Apt => "apt-get",
            _ => self.name(),
        }
    }

    fn installation_command(self, package_name: &str) -> (String, Vec<String>) {
        let args: Vec<&str> = match self {
            Self::Apt => vec!["apt-get", "install", "-y", package_name],
            Self::Paru => vec!["-S", "--needed", "--noconfirm", package_name],
            Self::Yay => vec!["-S", "--needed", "--noconfirm", package_name],
            Self::Pacman => vec!["pacman", "-S", "--needed", "--noconfirm", package_name],
            Self::Dnf => vec!["dnf", "install", "-y", package_name],
            Self::Yum => vec!["yum", "install", "-y", package_name],
            Self::Zypper => vec!["zypper", "--non-interactive", "install", package_name],
            Self::Apk => vec!["apk", "add", package_name],
            Self::Xbps => vec!["xbps-install", "-Sy", package_name],
            Self::Emerge => vec!["emerge", package_name],
        };

        match self {
            Self::Paru | Self::Yay => (
                self.name().to_string(),
                args.into_iter().map(str::to_string).collect(),
            ),
            _ => (
                "sudo".to_string(),
                args.into_iter().map(str::to_string).collect(),
            ),
        }
    }
}

pub struct HyprSys {
    home_dir: PathBuf,
}

impl HyprSys {
    pub fn new() -> Self {
        let home_dir = std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/home/aryan"));
        Self { home_dir }
    }

    async fn detect_distribution(&self) -> LinuxDistribution {
        for path in ["/etc/os-release", "/usr/lib/os-release"] {
            if let Ok(contents) = read_to_string(path).await {
                return LinuxDistribution::from_os_release(&contents);
            }
        }

        LinuxDistribution::unknown()
    }

    fn package_manager_candidates(distribution: &LinuxDistribution) -> &'static [PackageManager] {
        if distribution.belongs_to(&["arch", "manjaro", "endeavouros", "garuda"]) {
            &[
                PackageManager::Paru,
                PackageManager::Yay,
                PackageManager::Pacman,
            ]
        } else if distribution.belongs_to(&["debian", "ubuntu", "linuxmint", "pop", "kali"]) {
            &[PackageManager::Apt]
        } else if distribution.belongs_to(&[
            "fedora",
            "rhel",
            "centos",
            "rocky",
            "almalinux",
            "nobara",
        ]) {
            &[PackageManager::Dnf, PackageManager::Yum]
        } else if distribution.belongs_to(&["opensuse", "suse"]) {
            &[PackageManager::Zypper]
        } else if distribution.belongs_to(&["alpine"]) {
            &[PackageManager::Apk]
        } else if distribution.belongs_to(&["void"]) {
            &[PackageManager::Xbps]
        } else if distribution.belongs_to(&["gentoo"]) {
            &[PackageManager::Emerge]
        } else {
            &[
                PackageManager::Apt,
                PackageManager::Paru,
                PackageManager::Yay,
                PackageManager::Pacman,
                PackageManager::Dnf,
                PackageManager::Yum,
                PackageManager::Zypper,
                PackageManager::Apk,
                PackageManager::Xbps,
                PackageManager::Emerge,
            ]
        }
    }

    async fn detect_package_manager(
        &self,
        distribution: &LinuxDistribution,
    ) -> Option<PackageManager> {
        for package_manager in Self::package_manager_candidates(distribution) {
            if self
                .check_command_installed(package_manager.probe_binary())
                .await
            {
                return Some(*package_manager);
            }
        }

        None
    }

    fn is_supported_config_path(relative_path: &Path) -> bool {
        fn normal_component<'a>(component: std::path::Component<'a>) -> Option<&'a str> {
            match component {
                std::path::Component::Normal(value) => value.to_str(),
                _ => None,
            }
        }

        let components: Vec<_> = relative_path.components().collect();
        if components.len() < 3 {
            return false;
        }

        if normal_component(components[0]) != Some(".config") {
            return false;
        }

        matches!(
            normal_component(components[1]),
            Some("hypr" | "waybar" | "rofi" | "dunst" | "ags" | "quickshell" | "quick-shell" | "hyprlock" | "swaylock" | "waylock" | "swaync")
        ) && components[2..]
            .iter()
            .all(|component| normal_component(*component).is_some())
    }

    /// Verifies that AI-selected destinations are configuration files the agent owns.
    fn validate_path_safety(&self, relative_path: &Path) -> Result<PathBuf, String> {
        if !Self::is_supported_config_path(relative_path) {
            return Err(format!(
                "Refusing unsupported config destination '{}'. Allowed roots: .config/hypr, .config/waybar, .config/rofi, .config/dunst, .config/ags, .config/quickshell, .config/quick-shell, .config/hyprlock, .config/swaylock, .config/waylock, .config/swaync",
                relative_path.display()
            ));
        }

        let joined = self.home_dir.join(relative_path);

        let mut current = self.home_dir.clone();
        for component in relative_path.components() {
            current.push(component);
            match std::fs::symlink_metadata(&current) {
                Ok(metadata) if metadata.file_type().is_symlink() => {
                    return Err(format!(
                        "Refusing symbolic-link config destination '{}'",
                        relative_path.display()
                    ));
                }
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
                Err(error) => {
                    return Err(format!(
                        "Failed to inspect config destination '{}': {}",
                        relative_path.display(),
                        error
                    ));
                }
            }
        }

        // Lexically resolve ".." to prevent folder breakout attacks
        let mut components = std::collections::VecDeque::new();
        for comp in joined.components() {
            match comp {
                std::path::Component::ParentDir => {
                    components.pop_back();
                }
                std::path::Component::Normal(c) => {
                    components.push_back(c);
                }
                std::path::Component::RootDir => {
                    components.clear();
                }
                _ => {}
            }
        }

        let mut resolved = PathBuf::new();
        if joined.is_absolute() {
            resolved.push("/");
        }
        for c in components {
            resolved.push(c);
        }

        if resolved.starts_with(&self.home_dir) {
            Ok(resolved)
        } else {
            Err(format!(
                "Security violation: path '{}' escapes home directory boundaries!",
                resolved.display()
            ))
        }
    }

    async fn backup_existing_config(
        &self,
        target_path: &Path,
        relative_path: &Path,
    ) -> Result<Option<PathBuf>, Box<dyn std::error::Error + Send + Sync>> {
        if metadata(target_path).await.is_err() {
            return Ok(None);
        }

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_nanos()
            .to_string();
        let backup_path = self
            .home_dir
            .join(".local/share/dotengine/backups")
            .join(timestamp)
            .join(relative_path);

        if let Some(parent) = backup_path.parent() {
            create_dir_all(parent).await?;
        }
        copy(target_path, &backup_path).await?;

        Ok(Some(backup_path))
    }

    async fn check_font_awesome_installed(&self) -> bool {
        let output = Command::new("fc-list")
            .arg(":")
            .arg("family")
            .output()
            .await;

        if let Ok(out) = output {
            let list = String::from_utf8_lossy(&out.stdout).to_string();
            list.to_lowercase().contains("awesome")
        } else {
            false
        }
    }
}

#[async_trait]
impl SystemManager for HyprSys {
    fn get_home_directory(&self) -> PathBuf {
        self.home_dir.clone()
    }

    async fn detect_system_context(
        &self,
    ) -> Result<SystemContext, Box<dyn std::error::Error + Send + Sync>> {
        let distribution = self.detect_distribution().await;
        let package_manager = self
            .detect_package_manager(&distribution)
            .await
            .map(|manager| manager.name().to_string());

        // Identify active monitors by calling hyprctl monitors -j
        let mut monitors = Vec::new();
        let hyprctl_output = Command::new("hyprctl")
            .args(&["monitors", "-j"])
            .output()
            .await;

        if let Ok(output) = hyprctl_output {
            if output.status.success() {
                if let Ok(json_str) = String::from_utf8(output.stdout) {
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json_str) {
                        if let Some(arr) = parsed.as_array() {
                            for m in arr {
                                let name = m["name"].as_str().unwrap_or("Unknown").to_string();
                                let width = m["width"].as_u64().unwrap_or(1920) as u32;
                                let height = m["height"].as_u64().unwrap_or(1080) as u32;
                                let refresh_rate = m["refreshRate"].as_f64().unwrap_or(60.0) as f32;
                                let scale = m["scale"].as_f64().unwrap_or(1.0) as f32;

                                monitors.push(MonitorInfo {
                                    name,
                                    width,
                                    height,
                                    refresh_rate,
                                    scale,
                                });
                            }
                        }
                    }
                }
            }
        }

        // Default monitor details if hyprctl fails or isn't active
        if monitors.is_empty() {
            monitors.push(MonitorInfo {
                name: "HDMI-A-1".to_string(),
                width: 1920,
                height: 1080,
                refresh_rate: 60.0,
                scale: 1.0,
            });
        }

        // Detect dependency status of tools we care about
        let mut package_status = HashMap::new();
        for tool in &[
            "ags",
            "rofi",
            "dunst",
            "waybar",
            "hyprpaper",
            "kitty",
            "foot",
            "quick-shell",
            "hyprlock",
            "swaylock",
            "waylock",
        ] {
            let installed = self.check_command_installed(tool).await;
            package_status.insert(tool.to_string(), installed);
        }

        // Add Font Awesome check
        let font_awesome_installed = self.check_font_awesome_installed().await;
        package_status.insert("fonts-font-awesome".to_string(), font_awesome_installed);

        let mut context = SystemContext::new(distribution.display_name, package_manager);
        context.monitors = monitors;
        context.package_status = package_status;

        Ok(context)
    }

    async fn check_command_installed(&self, command: &str) -> bool {
        let output = Command::new("which").arg(command).output().await;

        match output {
            Ok(out) => out.status.success(),
            Err(_) => false,
        }
    }

    async fn install_package(
        &self,
        package_name: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let distribution = self.detect_distribution().await;
        let package_manager = match self.detect_package_manager(&distribution).await {
            Some(manager) => manager,
            None => {
                return Err(format!(
                    "No supported package manager found for {}",
                    distribution.display_name
                )
                .into())
            }
        };

        println!(
            "\n{} Required dependency is missing: {}",
            accent("Dotengine"),
            package_name
        );
        println!(
            "{} Detected Linux distribution: {}",
            accent("Dotengine"),
            distribution.display_name
        );
        println!(
            "{} Detected package manager: {}",
            accent("Dotengine"),
            package_manager.name()
        );
        println!("{} Run the installation? [y/N]", accent("Dotengine"));

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let choice = input.trim().to_lowercase();

        if choice == "y" || choice == "yes" {
            let translated_package = if package_name == "fonts-font-awesome" {
                match package_manager {
                    PackageManager::Pacman | PackageManager::Paru | PackageManager::Yay => "otf-font-awesome",
                    PackageManager::Apt => "fonts-font-awesome",
                    PackageManager::Dnf | PackageManager::Yum => "fontawesome-fonts",
                    _ => "fonts-font-awesome",
                }
            } else if package_name == "ags" {
                match package_manager {
                    PackageManager::Pacman | PackageManager::Paru | PackageManager::Yay => "aylurs-gtk-shell",
                    _ => "ags",
                }
            } else {
                package_name
            };

            let (program, args) = package_manager.installation_command(translated_package);
            let mut cmd = Command::new(program);
            cmd.args(args);

            let status = cmd.status().await?;
            if status.success() {
                println!(
                    "{} Successfully installed {}",
                    accent("Dotengine"),
                    package_name
                );
                Ok(())
            } else {
                Err(format!(
                    "Installation command exited with non-zero code for package {}",
                    package_name
                )
                .into())
            }
        } else {
            Err(format!("User aborted installation of dependency: {}", package_name).into())
        }
    }

    async fn confirm_config_changes(
        &self,
        configs: &[ConfigFile],
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        if configs.is_empty() {
            return Err("AI service returned no configuration files to apply".into());
        }

        println!("\n{}", heading("Dotengine proposed configuration changes"));
        let mut proposed_paths = HashSet::new();
        for config in configs {
            if !proposed_paths.insert(config.relative_path.clone()) {
                return Err(format!(
                    "AI service returned duplicate config destination '{}'",
                    config.relative_path.display()
                )
                .into());
            }

            let safe_path = self.validate_path_safety(&config.relative_path)?;
            let action = if metadata(&safe_path).await.is_ok() {
                "overwrite (backup will be created)"
            } else {
                "create"
            };
            println!(
                "    - ~/{} [{}; {} bytes]",
                config.relative_path.display(),
                action,
                config.content.len()
            );
        }

        print!(
            "{} Apply these configuration changes? [y/N] ",
            accent("Dotengine")
        );
        std::io::stdout().flush()?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        Ok(matches!(input.trim().to_lowercase().as_str(), "y" | "yes"))
    }

    async fn write_config_file(
        &self,
        file: &ConfigFile,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let safe_path = self.validate_path_safety(&file.relative_path)?;

        if let Some(backup_path) = self
            .backup_existing_config(&safe_path, &file.relative_path)
            .await?
        {
            println!(
                "    - Backed up existing config to '{}'",
                backup_path.display()
            );
        }

        // Ensure parent directories exist
        if let Some(parent) = safe_path.parent() {
            create_dir_all(parent).await?;
        }

        write(&safe_path, &file.content).await?;
        Ok(())
    }

    async fn read_config_file(
        &self,
        relative_path: &Path,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let safe_path = self.validate_path_safety(relative_path)?;
        let content = read_to_string(&safe_path).await?;
        Ok(content)
    }

    async fn verify_and_reload(&self, configs: &[ConfigFile]) -> Result<(), ErrorPayload> {
        // Execute hyprctl reload to apply Hyprland config files
        let output = Command::new("hyprctl").arg("reload").output().await;

        match output {
            Ok(out) => {
                if !out.status.success() {
                    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&out.stderr).to_string();

                    return Err(ErrorPayload::new(
                        "hyprctl reload".to_string(),
                        out.status.code(),
                        stdout,
                        stderr,
                        configs.to_vec(),
                    ));
                }

                // Check for specific configuration warnings/errors using hyprctl configerrors
                if let Ok(err_out) = Command::new("hyprctl").arg("configerrors").output().await {
                    let stdout_str = String::from_utf8_lossy(&err_out.stdout).to_string();
                    let trimmed = stdout_str.trim();
                    if !trimmed.is_empty() && trimmed != "no errors" {
                        return Err(ErrorPayload::new(
                            "hyprctl configerrors".to_string(),
                            Some(0),
                            "".to_string(),
                            stdout_str,
                            configs.to_vec(),
                        ));
                    }
                }

                // === SERVICE DAEMONIZATION ===
                // If Waybar was configured but isn't running, start it in the background
                let waybar_running = Command::new("pgrep").arg("waybar").status().await.is_ok_and(|s| s.success());
                if !waybar_running {
                    let has_waybar = configs.iter().any(|c| c.relative_path.to_string_lossy().contains("waybar"));
                    if has_waybar {
                        println!("{} Waybar is not running. Launching in background...", accent("Dotengine"));
                        let _ = Command::new("waybar")
                            .stdout(Stdio::null())
                            .stderr(Stdio::null())
                            .spawn();
                    }
                }

                // If Dunst was configured but isn't running, start it in the background
                let dunst_running = Command::new("pgrep").arg("dunst").status().await.is_ok_and(|s| s.success());
                if !dunst_running {
                    let has_dunst = configs.iter().any(|c| c.relative_path.to_string_lossy().contains("dunst"));
                    if has_dunst {
                        println!("{} Dunst is not running. Launching in background...", accent("Dotengine"));
                        let _ = Command::new("dunst")
                            .stdout(Stdio::null())
                            .stderr(Stdio::null())
                            .spawn();
                    }
                }

                Ok(())
            }
            Err(e) => {
                // If hyprctl itself isn't installed or running (e.g., in headless test environments),
                // we treat it as an error payload to trigger self-healing diagnostic recovery
                Err(ErrorPayload::new(
                    "hyprctl reload".to_string(),
                    None,
                    "".to_string(),
                    format!("Failed to spawn command process: {}", e),
                    configs.to_vec(),
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{HyprSys, LinuxDistribution, PackageManager};
    use crate::domain::ConfigFile;
    use crate::ports::SystemManager;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parses_distribution_metadata() {
        let distribution = LinuxDistribution::from_os_release(
            "PRETTY_NAME=\"Ubuntu 24.04 LTS\"\nID='ubuntu'\nID_LIKE=\"debian\"\n",
        );

        assert_eq!(distribution.display_name, "Ubuntu 24.04 LTS");
        assert_eq!(distribution.id, "ubuntu");
        assert_eq!(distribution.id_like, vec!["debian"]);
    }

    #[test]
    fn maps_distribution_families_to_native_package_managers() {
        let ubuntu = LinuxDistribution::from_os_release("ID=ubuntu\nID_LIKE=debian\n");
        let arch = LinuxDistribution::from_os_release("ID=endeavouros\nID_LIKE=arch\n");
        let fedora = LinuxDistribution::from_os_release("ID=fedora\n");

        assert_eq!(
            HyprSys::package_manager_candidates(&ubuntu),
            &[PackageManager::Apt]
        );
        assert_eq!(
            HyprSys::package_manager_candidates(&arch),
            &[
                PackageManager::Paru,
                PackageManager::Yay,
                PackageManager::Pacman
            ]
        );
        assert_eq!(
            HyprSys::package_manager_candidates(&fedora),
            &[PackageManager::Dnf, PackageManager::Yum]
        );
    }

    #[test]
    fn constructs_native_install_commands() {
        assert_eq!(
            PackageManager::Apt.installation_command("rofi"),
            (
                "sudo".to_string(),
                vec![
                    "apt-get".to_string(),
                    "install".to_string(),
                    "-y".to_string(),
                    "rofi".to_string()
                ]
            )
        );
        assert_eq!(
            PackageManager::Dnf.installation_command("waybar"),
            (
                "sudo".to_string(),
                vec![
                    "dnf".to_string(),
                    "install".to_string(),
                    "-y".to_string(),
                    "waybar".to_string()
                ]
            )
        );
    }

    #[test]
    fn only_accepts_supported_desktop_config_destinations() {
        assert!(HyprSys::is_supported_config_path(
            PathBuf::from(".config/hypr/hyprland.conf").as_path()
        ));
        assert!(HyprSys::is_supported_config_path(
            PathBuf::from(".config/waybar/style.css").as_path()
        ));
        assert!(!HyprSys::is_supported_config_path(
            PathBuf::from(".config/systemd/user/service").as_path()
        ));
        assert!(!HyprSys::is_supported_config_path(
            PathBuf::from(".config/hypr/../../../.bashrc").as_path()
        ));
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinked_supported_destination() {
        use std::os::unix::fs::symlink;

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let home_dir = std::env::temp_dir().join(format!("dotengine-symlink-test-{}", unique));
        let config_dir = home_dir.join(".config/hypr");
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::write(home_dir.join(".bashrc"), "do not overwrite").unwrap();
        symlink(home_dir.join(".bashrc"), config_dir.join("hyprland.conf")).unwrap();

        let system = HyprSys {
            home_dir: home_dir.clone(),
        };
        assert!(system
            .validate_path_safety(PathBuf::from(".config/hypr/hyprland.conf").as_path())
            .is_err());

        std::fs::remove_dir_all(home_dir).unwrap();
    }

    #[tokio::test]
    async fn backs_up_existing_supported_config_before_overwriting() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let home_dir = std::env::temp_dir().join(format!("dotengine-backup-test-{}", unique));
        let system = HyprSys {
            home_dir: home_dir.clone(),
        };
        let original = ConfigFile::new(
            ".config/hypr/hyprland.conf",
            "general { gaps_in = 5 }".to_string(),
        );
        let replacement = ConfigFile::new(
            ".config/hypr/hyprland.conf",
            "general { gaps_in = 8 }".to_string(),
        );

        system.write_config_file(&original).await.unwrap();
        system.write_config_file(&replacement).await.unwrap();

        let current = tokio::fs::read_to_string(home_dir.join(&replacement.relative_path))
            .await
            .unwrap();
        assert_eq!(current, replacement.content);

        let backups_root = home_dir.join(".local/share/dotengine/backups");
        let mut entries = tokio::fs::read_dir(backups_root).await.unwrap();
        let backup_run = entries.next_entry().await.unwrap().unwrap().path();
        let backed_up = tokio::fs::read_to_string(backup_run.join(&original.relative_path))
            .await
            .unwrap();
        assert_eq!(backed_up, original.content);

        tokio::fs::remove_dir_all(home_dir).await.unwrap();
    }
}
