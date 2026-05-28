use async_trait::async_trait;
use dotengine::application::{GenerationWorkflow, HealingWorkflow};
use dotengine::domain::{ConfigFile, ErrorPayload, MonitorInfo, SystemContext, UserPrompt};
use dotengine::ports::{AiService, SystemManager};
use std::path::{Path, PathBuf};
use std::sync::Arc;

// Mock AI Service implementation
struct MockAi {
    #[allow(dead_code)]
    fail_first: bool,
}

#[async_trait]
impl AiService for MockAi {
    async fn generate_config(
        &self,
        _prompt: &UserPrompt,
        _system_context: &SystemContext,
        _design_rules: &str,
    ) -> Result<Vec<ConfigFile>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(vec![ConfigFile::new(
            ".config/hypr/hyprland.conf",
            "gaps_in = 5".to_string(),
        )])
    }

    async fn repair_config(
        &self,
        _error_payload: &ErrorPayload,
        _design_rules: &str,
    ) -> Result<Vec<ConfigFile>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(vec![ConfigFile::new(
            ".config/hypr/hyprland.conf",
            "gaps_in = 6".to_string(),
        )])
    }
}

// Mock System Manager implementation
struct MockSys {
    home_dir: PathBuf,
    fail_verification: bool,
    verification_count: std::sync::atomic::AtomicUsize,
}

#[async_trait]
impl SystemManager for MockSys {
    fn get_home_directory(&self) -> PathBuf {
        self.home_dir.clone()
    }

    async fn detect_system_context(
        &self,
    ) -> Result<SystemContext, Box<dyn std::error::Error + Send + Sync>> {
        let mut context = SystemContext::new("Arch Linux".to_string(), Some("pacman".to_string()));
        context = context.with_monitor(MonitorInfo {
            name: "eDP-1".to_string(),
            width: 1920,
            height: 1080,
            refresh_rate: 60.0,
            scale: 1.0,
        });
        Ok(context)
    }

    async fn check_command_installed(&self, _command: &str) -> bool {
        true
    }

    async fn install_package(
        &self,
        _package_name: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    async fn confirm_config_changes(
        &self,
        _configs: &[ConfigFile],
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        Ok(true)
    }

    async fn write_config_file(
        &self,
        _file: &ConfigFile,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    async fn read_config_file(
        &self,
        _relative_path: &Path,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        Ok("gaps_in = 5".to_string())
    }

    async fn verify_and_reload(&self, configs: &[ConfigFile]) -> Result<(), ErrorPayload> {
        let count = self
            .verification_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if self.fail_verification && count == 0 {
            Err(ErrorPayload::new(
                "hyprctl reload".to_string(),
                Some(1),
                "".to_string(),
                "error: invalid syntax on line 1".to_string(),
                configs.to_vec(),
            ))
        } else {
            Ok(())
        }
    }
}

#[test]
fn test_config_file_path_safety() {
    let home = Path::new("/home/aryan");

    // Safe relative path
    let safe_file = ConfigFile::new(".config/hypr/hyprland.conf", "".to_string());
    let resolved = safe_file.resolve_safe_path(home);
    assert!(resolved.is_ok());
    assert_eq!(
        resolved.unwrap(),
        PathBuf::from("/home/aryan/.config/hypr/hyprland.conf")
    );

    // Traversal breakout attempt
    let breakout_file = ConfigFile::new("../../../etc/shadow", "".to_string());
    let resolved_breakout = breakout_file.resolve_safe_path(home);
    assert!(resolved_breakout.is_err());
}

#[tokio::test]
async fn test_successful_generation_workflow() {
    let ai = Arc::new(MockAi { fail_first: false });
    let sys = Arc::new(MockSys {
        home_dir: PathBuf::from("/home/aryan"),
        fail_verification: false,
        verification_count: std::sync::atomic::AtomicUsize::new(0),
    });

    let healing = HealingWorkflow::new(ai.clone(), sys.clone(), 3);
    let generation = GenerationWorkflow::new(ai.clone(), sys.clone(), healing);

    let res = generation
        .execute(
            "Modern Glass theme".to_string(),
            vec![],
            Some(0),
            "Use gaps",
            Some("SUPER, D"),
            "ags",
            "rofi",
            "hyprpaper",
            "hyprlock",
            "minimal violet theme",
            "centered analog clock",
            "swaync",
            true,
            true,
        )
        .await;

    assert!(res.is_ok());
    let configs = res.unwrap();
    assert_eq!(configs.len(), 1);
    assert_eq!(
        configs[0].relative_path.to_str().unwrap(),
        ".config/hypr/hyprland.conf"
    );
}

#[tokio::test]
async fn test_self_healing_recovery_workflow() {
    let ai = Arc::new(MockAi { fail_first: true });
    let sys = Arc::new(MockSys {
        home_dir: PathBuf::from("/home/aryan"),
        fail_verification: true,
        verification_count: std::sync::atomic::AtomicUsize::new(0),
    });

    let healing = HealingWorkflow::new(ai.clone(), sys.clone(), 3);
    let generation = GenerationWorkflow::new(ai.clone(), sys.clone(), healing);

    let res = generation
        .execute(
            "Modern Glass theme".to_string(),
            vec![],
            Some(0),
            "Use gaps",
            Some("SUPER, D"),
            "ags",
            "rofi",
            "hyprpaper",
            "hyprlock",
            "minimal violet theme",
            "centered analog clock",
            "swaync",
            true,
            true,
        )
        .await;

    assert!(res.is_ok());
    let configs = res.unwrap();
    assert_eq!(configs.len(), 1);
    // Config should have been repaired from mock repair output
    assert_eq!(configs[0].content, "gaps_in = 6");
}
