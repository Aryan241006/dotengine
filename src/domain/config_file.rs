use std::path::{Path, PathBuf};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConfigFile {
    /// Target path relative to the user's home directory (e.g., ".config/hypr/hyprland.conf")
    pub relative_path: PathBuf,
    /// Raw textual content of the configuration file
    pub content: String,
}

impl ConfigFile {
    pub fn new<P: Into<PathBuf>>(relative_path: P, content: String) -> Self {
        Self {
            relative_path: relative_path.into(),
            content,
        }
    }

    /// Safely resolves the absolute target path given the user's home directory path,
    /// verifying that the path does not escape the home folder structure.
    pub fn resolve_safe_path(&self, home_dir: &Path) -> Result<PathBuf, String> {
        let joined = home_dir.join(&self.relative_path);

        // Canonicalize or check parent structures to prevent path traversal directory breakouts (e.g., ../../../etc)
        let resolved = match joined.canonicalize() {
            Ok(p) => p,
            Err(_) => {
                // If it doesn't exist yet, we manually check using path components
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
                            // Reset if it's absolute
                            components.clear();
                        }
                        _ => {}
                    }
                }
                let mut p = PathBuf::new();
                if joined.is_absolute() {
                    p.push("/");
                }
                for c in components {
                    p.push(c);
                }
                p
            }
        };

        if resolved.starts_with(home_dir) {
            Ok(resolved)
        } else {
            Err(format!(
                "Security violation: path '{}' escapes the home directory '{}'",
                resolved.display(),
                home_dir.display()
            ))
        }
    }
}
