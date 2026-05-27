use crate::ui::{accent, heading};
use serde::{Deserialize, Serialize};
use std::fs::{create_dir_all, read_to_string, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiProvider {
    Gemini,
    Openai,
}

impl AiProvider {
    pub fn from_model_name(model: &str) -> Result<Self, String> {
        match model.to_lowercase().as_str() {
            "gemini" => Ok(Self::Gemini),
            "openai" => Ok(Self::Openai),
            _ => Err(format!(
                "Unsupported AI service '{}'. Choose 'gemini' or 'openai'.",
                model
            )),
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Gemini => "Gemini",
            Self::Openai => "OpenAI",
        }
    }

    fn setting_name(self) -> &'static str {
        match self {
            Self::Gemini => "gemini",
            Self::Openai => "openai",
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct StoredCredentials {
    preferred_provider: Option<String>,
    gemini_api_key: Option<String>,
    openai_api_key: Option<String>,
}

impl StoredCredentials {
    fn key_for(&self, provider: AiProvider) -> Option<&str> {
        match provider {
            AiProvider::Gemini => self.gemini_api_key.as_deref(),
            AiProvider::Openai => self.openai_api_key.as_deref(),
        }
        .filter(|key| !key.trim().is_empty())
    }

    fn set_key(&mut self, provider: AiProvider, key: String) {
        match provider {
            AiProvider::Gemini => self.gemini_api_key = Some(key),
            AiProvider::Openai => self.openai_api_key = Some(key),
        }
    }

    fn preferred_provider(&self) -> Option<AiProvider> {
        self.preferred_provider
            .as_deref()
            .and_then(|provider| AiProvider::from_model_name(provider).ok())
    }

    fn set_preferred_provider(&mut self, provider: AiProvider) {
        self.preferred_provider = Some(provider.setting_name().to_string());
    }
}

pub struct CredentialStore {
    path: PathBuf,
}

impl CredentialStore {
    pub fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let config_root = std::env::var_os("XDG_CONFIG_HOME")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME")
                    .filter(|value| !value.is_empty())
                    .map(|home| PathBuf::from(home).join(".config"))
            })
            .ok_or("Cannot store API credentials: neither XDG_CONFIG_HOME nor HOME is defined")?;

        Ok(Self {
            path: config_root.join("dotengine/credentials.json"),
        })
    }

    pub fn get_or_prompt(
        &self,
        provider: AiProvider,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let mut credentials = self.load()?;
        if let Some(key) = credentials.key_for(provider) {
            return Ok(key.to_string());
        }

        println!(
            "{} No stored {} API key found. Initial setup is required.",
            accent("Dotengine"),
            provider.display_name()
        );
        let key = Self::read_secret(&format!("Enter your {} API key: ", provider.display_name()))?;
        if key.trim().is_empty() {
            return Err("API key cannot be empty".into());
        }

        credentials.set_key(provider, key.clone());
        self.save(&credentials)?;
        println!(
            "{} Stored API key in {}.",
            accent("Dotengine"),
            self.path.display()
        );
        Ok(key)
    }

    pub fn select_provider(
        &self,
        requested_provider: Option<&str>,
    ) -> Result<AiProvider, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(requested) = requested_provider {
            return AiProvider::from_model_name(requested).map_err(Into::into);
        }

        let mut credentials = self.load()?;
        if let Some(provider) = credentials.preferred_provider() {
            return Ok(provider);
        }

        println!(
            "{}",
            heading("Welcome to Dotengine. Choose your AI provider:")
        );
        println!("  [1] Gemini");
        println!("  [2] OpenAI");
        print!("Enter choice [1-2]: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let provider = match input.trim() {
            "1" => AiProvider::Gemini,
            "2" => AiProvider::Openai,
            _ => return Err("Invalid provider choice. Enter 1 for Gemini or 2 for OpenAI.".into()),
        };

        credentials.set_preferred_provider(provider);
        self.save(&credentials)?;
        Ok(provider)
    }

    fn load(&self) -> Result<StoredCredentials, Box<dyn std::error::Error + Send + Sync>> {
        if !self.path.exists() {
            return Ok(StoredCredentials::default());
        }

        Ok(serde_json::from_str(&read_to_string(&self.path)?)?)
    }

    fn save(
        &self,
        credentials: &StoredCredentials,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(parent) = self.path.parent() {
            create_dir_all(parent)?;
        }

        let content = serde_json::to_vec_pretty(credentials)?;
        Self::write_private_file(&self.path, &content)?;
        Ok(())
    }

    #[cfg(unix)]
    fn write_private_file(path: &Path, contents: &[u8]) -> io::Result<()> {
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)?;
        file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
        file.write_all(contents)
    }

    #[cfg(not(unix))]
    fn write_private_file(path: &Path, contents: &[u8]) -> io::Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;
        file.write_all(contents)
    }

    fn read_secret(prompt: &str) -> Result<String, io::Error> {
        print!("{}", prompt);
        io::stdout().flush()?;

        #[cfg(unix)]
        let echo_disabled = Command::new("stty")
            .arg("-echo")
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success());

        let mut input = String::new();
        let result = io::stdin().read_line(&mut input);

        #[cfg(unix)]
        if echo_disabled {
            let _ = Command::new("stty")
                .arg("echo")
                .stderr(Stdio::null())
                .status();
            println!();
        }

        result?;
        Ok(input.trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{AiProvider, CredentialStore, StoredCredentials};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn stores_provider_keys_in_private_credentials_file() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("dotengine-creds-test-{}", unique));
        let path = root.join("credentials.json");
        let store = CredentialStore { path: path.clone() };
        let mut credentials = StoredCredentials::default();
        credentials.set_key(AiProvider::Gemini, "gem-key".to_string());
        credentials.set_key(AiProvider::Openai, "open-key".to_string());

        store.save(&credentials).unwrap();
        let loaded = store.load().unwrap();
        assert_eq!(loaded.preferred_provider(), None);
        assert_eq!(loaded.key_for(AiProvider::Gemini), Some("gem-key"));
        assert_eq!(loaded.key_for(AiProvider::Openai), Some("open-key"));

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn reads_stored_preferred_provider() {
        let mut credentials = StoredCredentials::default();
        credentials.set_preferred_provider(AiProvider::Openai);
        assert_eq!(credentials.preferred_provider(), Some(AiProvider::Openai));
    }
}
