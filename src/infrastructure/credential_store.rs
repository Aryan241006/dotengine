use crate::ui::{accent, heading};
use serde::{Deserialize, Serialize};
use std::fs::{create_dir_all, read_to_string, OpenOptions};
use std::io::{self, BufRead, Read, Write};
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
        interactive: bool,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let env_var_name = match provider {
            AiProvider::Gemini => "GEMINI_API_KEY",
            AiProvider::Openai => "OPENAI_API_KEY",
        };
        if let Ok(env_key) = std::env::var(env_var_name) {
            if !env_key.trim().is_empty() {
                return Ok(env_key.trim().to_string());
            }
        }

        let mut credentials = self.load()?;
        if let Some(key) = credentials.key_for(provider) {
            return Ok(key.to_string());
        }

        if !interactive {
            return Err(format!(
                "No stored {} API key found and the CLI is running in non-interactive mode.",
                provider.display_name()
            )
            .into());
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
        interactive: bool,
    ) -> Result<AiProvider, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(requested) = requested_provider {
            return AiProvider::from_model_name(requested).map_err(Into::into);
        }

        let mut credentials = self.load()?;
        if let Some(provider) = credentials.preferred_provider() {
            return Ok(provider);
        }

        if !interactive {
            return Err(
                "No saved AI provider preference found and the CLI is running in non-interactive mode."
                    .into(),
            );
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

    pub fn change_preferred_provider(
        &self,
        provider: AiProvider,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut credentials = self.load()?;
        credentials.set_preferred_provider(provider);
        self.save(&credentials)?;
        Ok(())
    }

    pub fn has_key(
        &self,
        provider: AiProvider,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let env_var_name = match provider {
            AiProvider::Gemini => "GEMINI_API_KEY",
            AiProvider::Openai => "OPENAI_API_KEY",
        };
        if let Ok(env_key) = std::env::var(env_var_name) {
            if !env_key.trim().is_empty() {
                return Ok(true);
            }
        }

        let credentials = self.load()?;
        Ok(credentials.key_for(provider).is_some())
    }

    pub fn prompt_and_save_provider(
        &self,
    ) -> Result<AiProvider, Box<dyn std::error::Error + Send + Sync>> {
        println!(
            "\n{}",
            heading("Choose your preferred default AI provider:")
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

        let mut credentials = self.load()?;
        credentials.set_preferred_provider(provider);
        self.save(&credentials)?;
        Ok(provider)
    }

    pub fn prompt_and_save_key(
        &self,
        provider: AiProvider,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let key = Self::read_secret(&format!("Enter your new {} API key: ", provider.display_name()))?;
        if key.trim().is_empty() {
            return Err("API key cannot be empty".into());
        }
        let mut credentials = self.load()?;
        credentials.set_key(provider, key.clone());
        self.save(&credentials)?;
        Ok(key)
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

    fn read_escape(handle: &mut io::StdinLock) -> io::Result<Option<String>> {
        let mut buf = [0u8; 1];
        handle.read_exact(&mut buf)?;
        if buf[0] != b'[' {
            return Ok(None);
        }
        let mut seq = String::from("[");
        loop {
            handle.read_exact(&mut buf)?;
            let c = buf[0] as char;
            seq.push(c);
            if c.is_ascii_alphabetic() || c == '~' {
                break;
            }
        }
        Ok(Some(seq))
    }

    fn read_secret(prompt: &str) -> Result<String, io::Error> {
        print!("{}", prompt);
        io::stdout().flush()?;

        #[cfg(unix)]
        let raw_enabled = Command::new("stty")
            .arg("raw")
            .arg("-echo")
            .stdin(Stdio::inherit())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success());

        let mut secret = String::new();
        let stdin = io::stdin();
        let mut handle = stdin.lock();
        let mut buffer = [0u8; 1];

        #[cfg(unix)]
        if raw_enabled {
            // Enable bracketed paste mode
            print!("\x1b[?2004h");
            let _ = io::stdout().flush();

            let mut in_bracketed_paste = false;

            loop {
                if handle.read_exact(&mut buffer).is_err() {
                    break;
                }
                let byte = buffer[0];

                if byte == 27 { // ESC
                    if let Ok(Some(seq)) = Self::read_escape(&mut handle) {
                        if seq == "[200~" {
                            in_bracketed_paste = true;
                        } else if seq == "[201~" {
                            in_bracketed_paste = false;
                        }
                    }
                    continue;
                }

                match byte {
                    b'\r' | b'\n' => {
                        if in_bracketed_paste {
                            continue;
                        }
                        print!("\r\n");
                        let _ = io::stdout().flush();
                        break;
                    }
                    3 | 4 => { // Ctrl+C or Ctrl+D
                        print!("\r\n");
                        let _ = io::stdout().flush();
                        // Disable bracketed paste
                        print!("\x1b[?2004l");
                        let _ = io::stdout().flush();
                        let _ = Command::new("stty")
                            .arg("-raw")
                            .arg("echo")
                            .stdin(Stdio::inherit())
                            .stderr(Stdio::null())
                            .status();
                        return Err(io::Error::new(io::ErrorKind::Interrupted, "Input cancelled"));
                    }
                    8 | 127 => { // Backspace
                        if !secret.is_empty() {
                            secret.pop();
                            print!("\x08 \x08");
                            let _ = io::stdout().flush();
                        }
                    }
                    other => {
                        if (32..127).contains(&other) {
                            secret.push(other as char);
                            print!("*");
                            let _ = io::stdout().flush();
                        }
                    }
                }
            }

            // Disable bracketed paste and restore terminal
            print!("\x1b[?2004l");
            let _ = io::stdout().flush();

            let _ = Command::new("stty")
                .arg("-raw")
                .arg("echo")
                .stdin(Stdio::inherit())
                .stderr(Stdio::null())
                .status();
        } else {
            let mut input = String::new();
            let mut handle = io::stdin().lock();
            let _ = handle.read_line(&mut input);
            secret = input.trim().to_string();
        }

        #[cfg(not(unix))]
        {
            let mut input = String::new();
            let mut handle = io::stdin().lock();
            let _ = handle.read_line(&mut input);
            secret = input.trim().to_string();
        }

        Ok(secret.trim().to_string())
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

    #[test]
    fn test_change_preferred_provider() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("dotengine-creds-test-{}", unique));
        let path = root.join("credentials.json");
        let store = CredentialStore { path: path.clone() };

        store.change_preferred_provider(AiProvider::Gemini).unwrap();
        let loaded = store.load().unwrap();
        assert_eq!(loaded.preferred_provider(), Some(AiProvider::Gemini));

        store.change_preferred_provider(AiProvider::Openai).unwrap();
        let loaded2 = store.load().unwrap();
        assert_eq!(loaded2.preferred_provider(), Some(AiProvider::Openai));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn test_change_api_key() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("dotengine-creds-test-{}", unique));
        let path = root.join("credentials.json");
        let store = CredentialStore { path: path.clone() };

        let mut credentials = StoredCredentials::default();
        credentials.set_key(AiProvider::Gemini, "old-key".to_string());
        store.save(&credentials).unwrap();

        let loaded = store.load().unwrap();
        assert_eq!(loaded.key_for(AiProvider::Gemini), Some("old-key"));

        let mut credentials = store.load().unwrap();
        credentials.set_key(AiProvider::Gemini, "new-key".to_string());
        store.save(&credentials).unwrap();

        let loaded_new = store.load().unwrap();
        assert_eq!(loaded_new.key_for(AiProvider::Gemini), Some("new-key"));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn test_env_var_key_lookup() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("dotengine-creds-test-{}", unique));
        let path = root.join("credentials.json");
        let store = CredentialStore { path };

        // Test with env vars unset/set
        std::env::remove_var("GEMINI_API_KEY");
        std::env::remove_var("OPENAI_API_KEY");

        assert!(!store.has_key(AiProvider::Gemini).unwrap());

        std::env::set_var("GEMINI_API_KEY", "env-gem-key");
        std::env::set_var("OPENAI_API_KEY", "env-open-key");

        assert!(store.has_key(AiProvider::Gemini).unwrap());
        assert!(store.has_key(AiProvider::Openai).unwrap());

        assert_eq!(store.get_or_prompt(AiProvider::Gemini, false).unwrap(), "env-gem-key");
        assert_eq!(store.get_or_prompt(AiProvider::Openai, false).unwrap(), "env-open-key");

        std::env::remove_var("GEMINI_API_KEY");
        std::env::remove_var("OPENAI_API_KEY");
    }
}

