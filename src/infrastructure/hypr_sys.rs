use crate::domain::{ConfigFile, ErrorPayload, MonitorInfo, SystemContext};
use crate::ports::SystemManager;
use crate::ui::{accent, heading};
use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{SystemTime, UNIX_EPOCH};
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

impl Default for HyprSys {
    fn default() -> Self {
        Self::new()
    }
}

impl HyprSys {
    pub fn new() -> Self {
        let home_dir = std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/home/aryan"));
        Self { home_dir }
    }

    pub async fn download_wallpaper(
        &self,
        query: &str,
        target_path: &Path,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("{} Querying Wallhaven API for: '{}'...", crate::ui::accent("Dotengine"), query);

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .user_agent("dotengine-rice/1.0")
            .build()?;

        let res = client
            .get("https://wallhaven.cc/api/v1/search")
            .query(&[("q", query), ("sorting", "relevance")])
            .send()
            .await?;

        if !res.status().is_success() {
            return Err(format!("Wallhaven search API returned status {}", res.status()).into());
        }

        let body: serde_json::Value = res.json().await?;
        let data = body.get("data")
            .and_then(|v| v.as_array())
            .ok_or("Invalid Wallhaven API response format")?;

        if data.is_empty() {
            return Err("No matching wallpaper found on Wallhaven".into());
        }

        let direct_url = data[0].get("path")
            .and_then(|v| v.as_str())
            .ok_or("Missing path field in Wallhaven search results")?;

        println!("{} Downloading high-resolution wallpaper from: {}...", crate::ui::accent("Dotengine"), direct_url);

        let img_res = client.get(direct_url).send().await?;
        if !img_res.status().is_success() {
            return Err(format!("Failed to download image from direct URL, status {}", img_res.status()).into());
        }

        let bytes = img_res.bytes().await?;

        if let Some(parent) = target_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(target_path, &bytes)?;
        Ok(())
    }


    async fn detect_distribution(&self) -> LinuxDistribution {
        for path in ["/etc/os-release", "/usr/lib/os-release"] {
            if let Ok(contents) = read_to_string(path).await {
                return LinuxDistribution::from_os_release(&contents);
            }
        }

        LinuxDistribution::unknown()
    }

    async fn detect_hyprland_version(&self) -> Option<(String, bool)> {
        let output = Command::new("hyprctl").arg("version").output().await.ok()?;
        if !output.status.success() {
            return None;
        }
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let version = stdout
            .lines()
            .find_map(|line| {
                let trimmed = line.trim();
                if trimmed.to_lowercase().starts_with("hyprland") {
                    trimmed.split_whitespace().nth(1).map(|s| s.to_string())
                } else {
                    None
                }
            })
            .or_else(|| {
                stdout
                    .split_whitespace()
                    .find(|part| part.chars().any(|c| c.is_ascii_digit()))
                    .map(|s| s.to_string())
            });

        let version = version?;
        let (major, minor) = Self::parse_version_parts(&version);
        let uses_lua = major > 0 || minor >= 55;

        Some((version, uses_lua))
    }

    fn parse_version_parts(version: &str) -> (u32, u32) {
        let mut parts = version.split('.');
        let major = parts
            .next()
            .and_then(|part| part.parse::<u32>().ok())
            .unwrap_or(0);
        let minor = parts
            .next()
            .and_then(|part| part.parse::<u32>().ok())
            .unwrap_or(0);
        (major, minor)
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
            Some(
                "hypr"
                    | "waybar"
                    | "rofi"
                    | "dunst"
                    | "ags"
                    | "quickshell"
                    | "quick-shell"
                    | "hypridle"
                    | "hyprlauncher"
                    | "hyprpicker"
                    | "hyprsunset"
                    | "hyprpolkitagent"
                    | "hyprsysteminfo"
                    | "hyprpwcenter"
                    | "hyprshutdown"
                    | "hyprtoolkit"
                    | "hyprland-guiutils"
                    | "hyprlock"
                    | "swaylock"
                    | "waylock"
                    | "swaync"
                    | "kitty"
                    | "foot"
                    | "alacritty"
                    | "wezterm"
                    | "ghostty"
                    | "fastfetch"
                    | "neofetch"
                    | "btop"
                    | "cava"
                    | "wlogout"
                    | "wofi"
            )
        ) && components[2..]
            .iter()
            .all(|component| normal_component(*component).is_some())
    }

    /// Verifies that AI-selected destinations are configuration files the agent owns.
    fn validate_path_safety(&self, relative_path: &Path) -> Result<PathBuf, String> {
        if !Self::is_supported_config_path(relative_path) {
            return Err(format!(
                "Refusing unsupported config destination '{}'. Allowed roots: .config/hypr, .config/waybar, .config/rofi, .config/dunst, .config/ags, .config/quickshell, .config/quick-shell, .config/hypridle, .config/hyprlauncher, .config/hyprpicker, .config/hyprsunset, .config/hyprpolkitagent, .config/hyprsysteminfo, .config/hyprpwcenter, .config/hyprshutdown, .config/hyprtoolkit, .config/hyprland-guiutils, .config/hyprlock, .config/swaylock, .config/waylock, .config/swaync, .config/kitty, .config/foot, .config/alacritty, .config/wezterm, .config/ghostty, .config/fastfetch, .config/neofetch, .config/btop, .config/cava, .config/wlogout, .config/wofi",
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

    async fn check_nerd_font_installed(&self) -> bool {
        let output = Command::new("fc-list")
            .arg(":")
            .arg("family")
            .output()
            .await;

        if let Ok(out) = output {
            let list = String::from_utf8_lossy(&out.stdout).to_string();
            list.to_lowercase().contains("nerd")
        } else {
            false
        }
    }

    fn resolve_install_package_name(package_name: &str, package_manager: PackageManager) -> String {
        match package_name {
            "fonts-font-awesome" => match package_manager {
                PackageManager::Pacman | PackageManager::Paru | PackageManager::Yay => {
                    "otf-font-awesome".to_string()
                }
                PackageManager::Apt => "fonts-font-awesome".to_string(),
                PackageManager::Dnf | PackageManager::Yum => "fontawesome-fonts".to_string(),
                _ => "fonts-font-awesome".to_string(),
            },
            "ags" => match package_manager {
                PackageManager::Pacman | PackageManager::Paru | PackageManager::Yay => {
                    "aylurs-gtk-shell".to_string()
                }
                _ => "ags".to_string(),
            },
            "swaync" => match package_manager {
                PackageManager::Apt => "sway-notification-center".to_string(),
                PackageManager::Dnf | PackageManager::Yum => "SwayNotificationCenter".to_string(),
                _ => "swaync".to_string(),
            },
            "network-manager-applet" => match package_manager {
                PackageManager::Apt => "network-manager-gnome".to_string(),
                _ => "network-manager-applet".to_string(),
            },
            other => other.to_string(),
        }
    }

    async fn launch_if_configured(binary: &str, token: &str, configs: &[ConfigFile]) {
        let running = Command::new("pgrep")
            .arg(binary)
            .status()
            .await
            .is_ok_and(|s| s.success());
        if running {
            return;
        }

        let wants_launch = configs.iter().any(|config| {
            config.relative_path.to_string_lossy().contains(token) || config.content.contains(token)
        });
        if wants_launch {
            println!(
                "{} {} is not running. Launching in background...",
                accent("Dotengine"),
                binary
            );
            let _ = Command::new(binary)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
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
            .args(["monitors", "-j"])
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
            "hypridle",
            "hyprpicker",
            "hyprlauncher",
            "hyprsunset",
            "hyprpolkitagent",
            "hyprsysteminfo",
            "hyprpwcenter",
            "hyprshutdown",
            "xdg-desktop-portal-hyprland",
            "kitty",
            "foot",
            "quick-shell",
            "hyprlock",
            "swaylock",
            "waylock",
            "wlogout",
        ] {

            let installed = self.check_command_installed(tool).await;
            package_status.insert(tool.to_string(), installed);
        }

        // Add Font Awesome check
        let font_awesome_installed = self.check_font_awesome_installed().await;
        let nerd_font_installed = self.check_nerd_font_installed().await;
        package_status.insert("fonts-font-awesome".to_string(), font_awesome_installed);
        package_status.insert("fonts-nerd-font".to_string(), nerd_font_installed);

        let mut context = SystemContext::new(distribution.display_name, package_manager);
        context.monitors = monitors;
        context.package_status = package_status;

        if let Some((version, uses_lua)) = self.detect_hyprland_version().await {
            context = context.with_hyprland_version(version, uses_lua);
        }

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
            let translated_package =
                Self::resolve_install_package_name(package_name, package_manager);

            let (program, args) = package_manager.installation_command(&translated_package);
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

        let content = if file.relative_path.to_string_lossy().contains("hyprpaper.conf") {
            file.content.replace("~", &self.home_dir.to_string_lossy())
        } else {
            file.content.clone()
        };

        write(&safe_path, &content).await?;

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

    async fn verify_and_reload(&self, configs: &[ConfigFile], wallpaper_query: Option<&str>) -> Result<(), ErrorPayload> {
        let mut wallpaper_path_str = None;
        // === AUTOMATIC WALLPAPER GENERATION AND PROVISIONING ===
        if let Some(hyprpaper_conf) = configs.iter().find(|c| c.relative_path.to_string_lossy().contains("hyprpaper.conf")) {
            for line in hyprpaper_conf.content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("preload") {
                    if let Some((_, path_val)) = trimmed.split_once('=') {
                        let raw_path = path_val.trim();
                        let stripped = raw_path.trim_matches(|c| c == '"' || c == '\'');
                        wallpaper_path_str = Some(stripped.replace("~", self.home_dir.to_string_lossy().as_ref()));
                        break;
                    }
                }
            }

            if let Some(ref path_str) = wallpaper_path_str {
                let path = PathBuf::from(path_str);
                if path.starts_with(&self.home_dir) && !path.exists() {
                    let mut downloaded_successfully = false;
                    if let Some(query) = wallpaper_query {
                        if !query.trim().is_empty() {
                            match self.download_wallpaper(query, &path).await {
                                Ok(_) => {
                                    println!("{} Reference design wallpaper successfully fetched and applied.", crate::ui::accent("Dotengine"));
                                    downloaded_successfully = true;
                                }
                                Err(e) => {
                                    println!("{} Failed to fetch reference wallpaper: {}. Falling back to gradient generator.", crate::ui::warning("Dotengine"), e);
                                }
                            }
                        }
                    }

                    if !downloaded_successfully {
                        println!("{} Wallpaper file is missing at '{}'. Generating matching aesthetic background...", crate::ui::accent("Dotengine"), path.display());

                    let mut colors = Vec::new();
                    for c in configs {
                        let mut i = 0;
                        let bytes = c.content.as_bytes();
                        while i + 6 < bytes.len() {
                            if bytes[i] == b'#' {
                                let mut is_hex = true;
                                let mut hex_str = String::new();
                                for j in 1..=6 {
                                    let ch = bytes[i + j] as char;
                                    if ch.is_ascii_hexdigit() {
                                        hex_str.push(ch);
                                    } else {
                                        is_hex = false;
                                        break;
                                    }
                                }
                                if is_hex {
                                    colors.push(hex_str);
                                }
                                i += 6;
                            }
                            i += 1;
                        }
                    }

                    let hex_to_rgb = |hex: &str| -> Option<[u8; 3]> {
                        if hex.len() != 6 {
                            return None;
                        }
                        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                        Some([r, g, b])
                    };

                    let mut rgb_colors: Vec<[u8; 3]> = colors.iter()
                        .filter_map(|hex| hex_to_rgb(hex))
                        .collect();

                    rgb_colors.sort_by_key(|rgb| {
                        (0.2126 * rgb[0] as f32 + 0.7152 * rgb[1] as f32 + 0.0722 * rgb[2] as f32) as u32
                    });
                    rgb_colors.dedup();

                    let (color1, color2) = if rgb_colors.len() >= 2 {
                        (rgb_colors[0], *rgb_colors.last().unwrap())
                    } else if rgb_colors.len() == 1 {
                        ([30, 30, 46], rgb_colors[0])
                    } else {
                        ([30, 30, 46], [137, 180, 250])
                    };

                    let width = 1920;
                    let height = 1080;
                    if let Some(parent) = path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }

                    let mut img: image::RgbImage = image::ImageBuffer::new(width, height);
                    for (x, y, pixel) in img.enumerate_pixels_mut() {
                        let t = (x as f32 / width as f32 + y as f32 / height as f32) / 2.0;
                        let r = ((1.0 - t) * color1[0] as f32 + t * color2[0] as f32) as u8;
                        let g = ((1.0 - t) * color1[1] as f32 + t * color2[1] as f32) as u8;
                        let b = ((1.0 - t) * color1[2] as f32 + t * color2[2] as f32) as u8;
                        *pixel = image::Rgb([r, g, b]);
                    }

                    if let Err(e) = img.save(&path) {
                        println!("{} Failed to save generated wallpaper: {}", crate::ui::warning("Dotengine"), e);
                    } else {
                        println!("{} Bespoke wallpaper successfully generated.", crate::ui::accent("Dotengine"));
                    }
                    }
                }
            }
        }

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

                // === DYNAMIC SERVICE RESTART & MUTUAL EXCLUSION ===
                let has_waybar = configs.iter().any(|c| c.relative_path.to_string_lossy().contains("waybar"));
                let has_ags = configs.iter().any(|c| c.relative_path.to_string_lossy().contains("ags"));
                let has_dunst = configs.iter().any(|c| c.relative_path.to_string_lossy().contains("dunst"));
                let has_swaync = configs.iter().any(|c| c.relative_path.to_string_lossy().contains("swaync"));
                let has_hyprpaper = configs.iter().any(|c| c.relative_path.to_string_lossy().contains("hyprpaper"));
                let has_hypridle = configs.iter().any(|c| c.relative_path.to_string_lossy().contains("hypridle"));

                // 1. Manage Panel Transition / Reload
                if has_waybar {
                    // Try smooth reload with SIGUSR2 first to avoid destroying tray icons
                    let reload_status = Command::new("pkill")
                        .args(["-USR2", "waybar"])
                        .status()
                        .await;
                    if reload_status.is_ok_and(|s| s.success()) {
                        println!("{} Seamlessly reloaded Waybar config via SIGUSR2.", accent("Dotengine"));
                        let _ = Command::new("pkill").arg("ags").status().await;
                    } else {
                        println!("{} Restarting Waybar panel to apply new configuration...", accent("Dotengine"));
                        let _ = Command::new("pkill").arg("waybar").status().await;
                        let _ = Command::new("pkill").arg("ags").status().await;
                        let _ = Command::new("waybar").stdout(Stdio::null()).stderr(Stdio::null()).spawn();
                    }
                } else if has_ags {
                    println!("{} Restarting AGS shell to apply new configuration...", accent("Dotengine"));
                    let _ = Command::new("pkill").arg("ags").status().await;
                    let _ = Command::new("pkill").arg("waybar").status().await;
                    let _ = Command::new("ags").stdout(Stdio::null()).stderr(Stdio::null()).spawn();
                }

                // 2. Manage Notification Center Transition / Reload
                if has_dunst {
                    println!("{} Restarting Dunst notification center to apply new configuration...", accent("Dotengine"));
                    let _ = Command::new("pkill").arg("dunst").status().await;
                    let _ = Command::new("pkill").arg("swaync").status().await;
                    let _ = Command::new("dunst").stdout(Stdio::null()).stderr(Stdio::null()).spawn();
                } else if has_swaync {
                    println!("{} Restarting SwayNC notification center to apply new configuration...", accent("Dotengine"));
                    let _ = Command::new("pkill").arg("swaync").status().await;
                    let _ = Command::new("pkill").arg("dunst").status().await;
                    let _ = Command::new("swaync").stdout(Stdio::null()).stderr(Stdio::null()).spawn();
                }

                // 3. Manage Wallpaper & Idle Transition
                if has_hyprpaper {
                    let mut smooth_reload_success = false;
                    if let Some(ref path_str) = wallpaper_path_str {
                        // Preload the wallpaper
                        let preload_status = Command::new("hyprctl")
                            .args(["hyprpaper", "preload", path_str])
                            .stdout(Stdio::null())
                            .stderr(Stdio::null())
                            .status()
                            .await;
                        if preload_status.is_ok_and(|s| s.success()) {
                            // Apply wallpaper to all monitors
                            let apply_status = Command::new("hyprctl")
                                .args(["hyprpaper", "wallpaper", &format!(",{}", path_str)])
                                .stdout(Stdio::null())
                                .stderr(Stdio::null())
                                .status()
                                .await;
                            if apply_status.is_ok_and(|s| s.success()) {
                                println!("{} Seamlessly applied new wallpaper via hyprctl IPC.", accent("Dotengine"));
                                smooth_reload_success = true;
                            }
                        }
                    }

                    if !smooth_reload_success {
                        println!("{} Restarting Hyprpaper to apply new wallpaper...", accent("Dotengine"));
                        let _ = Command::new("pkill").arg("hyprpaper").status().await;
                        let _ = Command::new("hyprpaper").stdout(Stdio::null()).stderr(Stdio::null()).spawn();
                    }
                }
                if has_hypridle {
                    println!("{} Restarting Hypridle daemon...", accent("Dotengine"));
                    let _ = Command::new("pkill").arg("hypridle").status().await;
                    let _ = Command::new("hypridle").stdout(Stdio::null()).stderr(Stdio::null()).spawn();
                }

                // For any other daemons not in active configs, keep standard fallback checks
                if !has_hyprpaper {
                    Self::launch_if_configured("hyprpaper", "hyprpaper", configs).await;
                }
                if !has_hypridle {
                    Self::launch_if_configured("hypridle", "hypridle", configs).await;
                }
                Self::launch_if_configured("hyprpolkitagent", "hyprpolkitagent", configs).await;
                Self::launch_if_configured("hyprsunset", "hyprsunset", configs).await;

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
    fn resolves_component_packages_for_distribution_variants() {
        assert_eq!(
            HyprSys::resolve_install_package_name("ags", PackageManager::Pacman),
            "aylurs-gtk-shell".to_string()
        );
        assert_eq!(
            HyprSys::resolve_install_package_name("hyprpaper", PackageManager::Apt),
            "hyprpaper".to_string()
        );
        assert_eq!(
            HyprSys::resolve_install_package_name("fonts-font-awesome", PackageManager::Dnf),
            "fontawesome-fonts".to_string()
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
        assert!(HyprSys::is_supported_config_path(
            PathBuf::from(".config/kitty/kitty.conf").as_path()
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
