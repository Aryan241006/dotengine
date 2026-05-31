use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::fs;

const DESIGN_SKILL_FALLBACK: &str = r#"# Hyprland Design Skill

Use clean spacing, complete startup wiring, and a coherent visual system. A setup is not finished unless it includes the panel, launcher, lockscreen, wallpaper flow, notification path, and any companion services needed for a barebones Hyprland session.
"#;

const HYPR_SKILL_FALLBACK: &str = r#"# Hyprland Syntax Skill

Follow the syntax family that matches the detected Hyprland version. Prefer explicit, reload-safe configuration, use the correct rule syntax for the release branch, and avoid placing application configs inside Hyprland's own config blocks.
"#;

const ECOSYSTEM_SKILL_FALLBACK: &str = r#"# Hypr Ecosystem Skill

Treat Hypr ecosystem tools as first-class desktop plumbing. If a setup uses wallpaper, lockscreen, idle handling, portals, polkit, or Qt theming support, wire the required configs and startup behavior explicitly.
"#;

pub const UPSTREAM_SOURCE_MANIFEST: &[&str] = &[
    "https://wiki.hypr.land/Configuring/",
    "https://wiki.hypr.land/Hypr-Ecosystem/",
    "https://wiki.hypr.land/Configuring/Example-configurations/",
];

#[derive(Debug, Clone)]
pub struct SkillCorpus {
    pub design: String,
    pub syntax: String,
    pub ecosystem: String,
    pub upstream_refresh: Option<String>,
}

impl SkillCorpus {
    pub fn load(workspace_root: &Path) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let design = load_skill_file(workspace_root, "design-skill.md", DESIGN_SKILL_FALLBACK)?;
        let syntax = load_skill_file(workspace_root, "skill.md", HYPR_SKILL_FALLBACK)?;
        let ecosystem = load_skill_file(
            workspace_root,
            "hypr-ecosystem-skill.md",
            ECOSYSTEM_SKILL_FALLBACK,
        )?;
        let upstream_refresh = load_cached_upstream_refresh();

        Ok(Self {
            design,
            syntax,
            ecosystem,
            upstream_refresh,
        })
    }

    pub fn combined(&self) -> String {
        let mut combined = format!(
            "{}\n\n=== HYPRLAND SYNTAX SKILL ===\n{}\n\n=== HYPR ECOSYSTEM SKILL ===\n{}",
            self.design, self.syntax, self.ecosystem
        );
        if let Some(refresh) = self.upstream_refresh.as_deref() {
            if !refresh.trim().is_empty() {
                combined.push_str("\n\n=== UPSTREAM REFRESH NOTES ===\n");
                combined.push_str(refresh);
            }
        }
        combined
    }

    pub fn reference_analysis_guidance(&self) -> String {
        let mut lines = vec![
            "=== REFERENCE ANALYSIS BRIEF ===".to_string(),
            "Use the attached screenshot(s) as the primary source of truth for the visible design.".to_string(),
            "Infer only the design language and the minimum desktop plumbing needed to make the setup feel complete.".to_string(),
            "Return compact JSON only; do not write config files in this step.".to_string(),
            "Prefer canonical lowercase component ids such as waybar, rofi, hyprpaper, hyprlock, swaync, dunst, ags, and quick-shell.".to_string(),
            "If a component or effect is uncertain, leave it null instead of inventing prose.".to_string(),
            "Treat wallpaper, lockscreen, idle handling, portals, polkit, and panel startup wiring as completeness signals for barebones Hyprland setups.".to_string(),
        ];

        if !self.design.trim().is_empty() {
            lines.push("Design skill anchor: complete setups should not feel hollow; they need the panel, launcher, lockscreen, wallpaper flow, notifications, and companion services.".to_string());
        }
        if !self.ecosystem.trim().is_empty() {
            lines.push("Ecosystem skill anchor: Hypr ecosystem tools are desktop plumbing and should be wired explicitly, not treated as optional decoration.".to_string());
        }

        lines.join("\n")
    }

    pub fn upstream_sources() -> &'static [&'static str] {
        UPSTREAM_SOURCE_MANIFEST
    }

    pub async fn refresh_upstream_cache() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let notes = fetch_upstream_refresh_notes().await?;
        if let Some(path) = upstream_refresh_cache_path() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).await?;
            }
            fs::write(path, notes).await?;
        }

        Ok(())
    }
}

pub fn load_skill_corpus(
    workspace_root: &Path,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    Ok(SkillCorpus::load(workspace_root)?.combined())
}

fn load_skill_file(
    workspace_root: &Path,
    file_name: &str,
    fallback: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let path = workspace_root.join(file_name);
    if path.exists() {
        Ok(std::fs::read_to_string(path)?)
    } else {
        tracing::warn!(
            "{} not found in project root. Falling back to default guidance.",
            file_name
        );
        Ok(fallback.to_string())
    }
}

fn user_data_root() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("XDG_DATA_HOME") {
        Some(PathBuf::from(path))
    } else {
        std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share"))
    }
}

fn upstream_refresh_cache_path() -> Option<PathBuf> {
    user_data_root().map(|root| root.join("dotengine/skill-corpus/upstream-refresh.md"))
}

fn load_cached_upstream_refresh() -> Option<String> {
    let path = upstream_refresh_cache_path()?;
    std::fs::read_to_string(path)
        .ok()
        .filter(|content| !content.trim().is_empty())
}

async fn fetch_upstream_refresh_notes() -> Result<String, Box<dyn std::error::Error + Send + Sync>>
{
    let client = reqwest::Client::builder()
        .user_agent("dotengine-skill-refresh/1.0")
        .timeout(std::time::Duration::from_secs(20))
        .build()?;

    let mut sections = Vec::new();
    for source in UPSTREAM_SOURCE_MANIFEST {
        let response = client.get(*source).send().await?;
        let body = response.text().await?;
        let text = html_to_text(&body);
        if !text.trim().is_empty() {
            sections.push(format!("## {}\n\n{}", source, text));
        }
    }

    let refreshed_at = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    Ok(format!(
        "# Upstream Hyprland Refresh\n\nRefreshed at unix={}\n\n{}\n",
        refreshed_at,
        sections.join("\n\n---\n\n")
    ))
}

fn html_to_text(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut in_script = false;
    let mut in_style = false;

    for (idx, ch) in html.char_indices() {
        if ch == '<' {
            let window: String = html[idx..].chars().take(10).collect();
            let lower = window.to_lowercase();
            if lower.starts_with("<script") {
                in_script = true;
            } else if lower.starts_with("</script") {
                in_script = false;
            } else if lower.starts_with("<style") {
                in_style = true;
            } else if lower.starts_with("</style") {
                in_style = false;
            } else if lower.starts_with("<br")
                || lower.starts_with("</p")
                || lower.starts_with("</div")
                || lower.starts_with("</li")
                || lower.starts_with("</h1")
                || lower.starts_with("</h2")
                || lower.starts_with("</h3")
                || lower.starts_with("</tr")
            {
                out.push('\n');
            }
            in_tag = true;
        } else if ch == '>' {
            in_tag = false;
        } else if !in_tag && !in_script && !in_style {
            out.push(ch);
        }
    }

    let decoded = out
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'");

    decoded
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::{load_skill_corpus, SkillCorpus};
    use std::fs;

    #[test]
    fn load_skill_corpus_combines_all_sections() {
        let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let corpus = SkillCorpus::load(workspace_root).expect("skill corpus should load");
        assert!(!corpus.design.trim().is_empty());
        assert!(!corpus.syntax.trim().is_empty());
        assert!(!corpus.ecosystem.trim().is_empty());

        let combined = load_skill_corpus(workspace_root).expect("combined corpus should load");
        assert!(combined.contains("HYPRLAND SYNTAX SKILL"));
        assert!(combined.contains("HYPR ECOSYSTEM SKILL"));
        assert!(combined.contains("Example-Config Pattern Library"));
    }

    #[test]
    fn load_skill_corpus_uses_fallbacks_when_missing() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_root = std::env::temp_dir().join(format!("dotengine-skill-corpus-{}", unique));
        fs::create_dir_all(&temp_root).unwrap();

        let corpus = SkillCorpus::load(&temp_root).expect("fallback corpus should load");
        assert!(corpus.design.contains("Hyprland Design Skill"));
        assert!(corpus.syntax.contains("Hyprland Syntax Skill"));
        assert!(corpus.ecosystem.contains("Hypr Ecosystem Skill"));
        assert!(corpus.upstream_refresh.is_none());

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn upstream_source_manifest_is_present() {
        assert!(SkillCorpus::upstream_sources()
            .iter()
            .any(|source| source.contains("Hypr-Ecosystem")));
    }

    #[test]
    fn html_refresh_notes_are_normalized() {
        let html = r#"
            <html>
              <head><style>.x { color: red; }</style></head>
              <body>
                <h1>Hypr</h1>
                <p>Keep <strong>startup</strong> wired.</p>
                <script>console.log('ignore');</script>
              </body>
            </html>
        "#;

        let text = super::html_to_text(html);
        assert!(text.contains("Hypr"));
        assert!(text.contains("Keep"));
        assert!(text.contains("startup"));
        assert!(!text.contains("console.log"));
        assert!(!text.contains(".x { color: red; }"));
    }
}
