use crate::application::normalize_component_value;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct DesignStackHints {
    pub panel: Option<String>,
    pub launcher: Option<String>,
    pub wallpaper: Option<String>,
    pub lockscreen: Option<String>,
    pub notification: Option<String>,
}

impl DesignStackHints {
    pub fn normalize(&self) -> Self {
        Self {
            panel: self
                .panel
                .as_deref()
                .and_then(|value| normalize_component_value("panel", value)),
            launcher: self
                .launcher
                .as_deref()
                .and_then(|value| normalize_component_value("launcher", value)),
            wallpaper: self
                .wallpaper
                .as_deref()
                .and_then(|value| normalize_component_value("wallpaper", value)),
            lockscreen: self
                .lockscreen
                .as_deref()
                .and_then(|value| normalize_component_value("lockscreen", value)),
            notification: self
                .notification
                .as_deref()
                .and_then(|value| normalize_component_value("notification", value)),
        }
    }

    pub fn apply_to_values(
        &self,
        panel: String,
        launcher: String,
        wallpaper: String,
        lockscreen: String,
        notification: String,
    ) -> (String, String, String, String, String) {
        let panel = self
            .panel
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(&panel)
            .to_string();
        let launcher = self
            .launcher
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(&launcher)
            .to_string();
        let wallpaper = self
            .wallpaper
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(&wallpaper)
            .to_string();
        let lockscreen = self
            .lockscreen
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(&lockscreen)
            .to_string();
        let notification = self
            .notification
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(&notification)
            .to_string();
        let panel = normalize_component_value("panel", &panel).unwrap_or(panel);
        let launcher = normalize_component_value("launcher", &launcher).unwrap_or(launcher);
        let wallpaper = normalize_component_value("wallpaper", &wallpaper).unwrap_or(wallpaper);
        let lockscreen = normalize_component_value("lockscreen", &lockscreen).unwrap_or(lockscreen);
        let notification =
            normalize_component_value("notification", &notification).unwrap_or(notification);
        (panel, launcher, wallpaper, lockscreen, notification)
    }

    pub fn is_empty(&self) -> bool {
        self.panel.as_deref().unwrap_or("").trim().is_empty()
            && self.launcher.as_deref().unwrap_or("").trim().is_empty()
            && self.wallpaper.as_deref().unwrap_or("").trim().is_empty()
            && self.lockscreen.as_deref().unwrap_or("").trim().is_empty()
            && self.notification.as_deref().unwrap_or("").trim().is_empty()
    }
}

#[derive(Debug, Clone, serde::Deserialize, Default)]
struct RawDesignStackHints {
    panel: Option<String>,
    launcher: Option<String>,
    wallpaper: Option<String>,
    lockscreen: Option<String>,
    notification: Option<String>,
}

impl RawDesignStackHints {
    fn into_stack(self) -> DesignStackHints {
        DesignStackHints {
            panel: self
                .panel
                .as_deref()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
            launcher: self
                .launcher
                .as_deref()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
            wallpaper: self
                .wallpaper
                .as_deref()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
            lockscreen: self
                .lockscreen
                .as_deref()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
            notification: self
                .notification
                .as_deref()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, Default)]
struct RawDesignReferenceSpec {
    summary: Option<String>,
    visual_style: Option<String>,
    palette: Option<String>,
    density: Option<String>,
    blur: Option<String>,
    transparency: Option<String>,
    rounding: Option<String>,
    stack: Option<RawDesignStackHints>,
    startup_commands: Option<serde_json::Value>,
    ecosystem_components: Option<serde_json::Value>,
    completion_notes: Option<serde_json::Value>,
    confidence: Option<serde_json::Value>,
    wallpaper_description: Option<String>,
}

impl RawDesignReferenceSpec {
    fn into_spec(self) -> DesignReferenceSpec {
        let startup_commands = match self.startup_commands {
            Some(serde_json::Value::Array(arr)) => arr.into_iter().filter_map(|v| v.as_str().map(String::from)).collect(),
            Some(serde_json::Value::String(s)) => vec![s],
            _ => Vec::new(),
        };
        let ecosystem_components = match self.ecosystem_components {
            Some(serde_json::Value::Array(arr)) => arr.into_iter().filter_map(|v| v.as_str().map(String::from)).collect(),
            Some(serde_json::Value::String(s)) => vec![s],
            _ => Vec::new(),
        };
        let completion_notes = match self.completion_notes {
            Some(serde_json::Value::Array(arr)) => arr.into_iter().filter_map(|v| v.as_str().map(String::from)).collect(),
            Some(serde_json::Value::String(s)) => vec![s],
            _ => Vec::new(),
        };

        let confidence = self.confidence.and_then(|val| {
            if let Some(num) = val.as_u64() {
                Some(num as u8)
            } else if let Some(s) = val.as_str() {
                s.chars().filter(|c| c.is_ascii_digit()).collect::<String>().parse::<u8>().ok()
            } else {
                None
            }
        });

        DesignReferenceSpec {
            summary: self.summary.unwrap_or_default(),
            visual_style: self.visual_style.unwrap_or_default(),
            palette: normalize_optional_text(self.palette),
            density: normalize_optional_text(self.density),
            blur: normalize_optional_text(self.blur),
            transparency: normalize_optional_text(self.transparency),
            rounding: normalize_optional_text(self.rounding),
            stack: self.stack.unwrap_or_default().into_stack(),
            startup_commands,
            ecosystem_components,
            completion_notes,
            confidence,
            wallpaper_description: normalize_optional_text(self.wallpaper_description),
        }
        .normalize()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct DesignReferenceSpec {
    pub summary: String,
    pub visual_style: String,
    pub palette: Option<String>,
    pub density: Option<String>,
    pub blur: Option<String>,
    pub transparency: Option<String>,
    pub rounding: Option<String>,
    pub stack: DesignStackHints,
    pub startup_commands: Vec<String>,
    pub ecosystem_components: Vec<String>,
    pub completion_notes: Vec<String>,
    pub confidence: Option<u8>,
    pub wallpaper_description: Option<String>,
}

impl DesignReferenceSpec {
    pub fn normalize(&self) -> Self {
        let mut ecosystem_components = self
            .ecosystem_components
            .iter()
            .map(|component| component.trim().to_lowercase())
            .filter(|component| !component.is_empty())
            .collect::<Vec<_>>();
        ecosystem_components.sort();
        ecosystem_components.dedup();

        Self {
            summary: self.summary.trim().to_string(),
            visual_style: self.visual_style.trim().to_string(),
            palette: self
                .palette
                .as_deref()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
            density: self
                .density
                .as_deref()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
            blur: self
                .blur
                .as_deref()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
            transparency: self
                .transparency
                .as_deref()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
            rounding: self
                .rounding
                .as_deref()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
            stack: self.stack.normalize(),
            startup_commands: self
                .startup_commands
                .iter()
                .map(|command| command.trim().to_string())
                .filter(|command| !command.is_empty())
                .collect(),
            ecosystem_components,
            completion_notes: self
                .completion_notes
                .iter()
                .map(|note| note.trim().to_string())
                .filter(|note| !note.is_empty())
                .collect(),
            confidence: self.confidence.map(|confidence| confidence.min(100)),
            wallpaper_description: self
                .wallpaper_description
                .as_deref()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
        }
    }

    pub fn to_guidelines(&self) -> String {
        let mut lines = vec![
            "=== REFERENCE DESIGN EXTRACTION ===".to_string(),
            format!("Summary: {}", self.summary),
            format!("Visual style: {}", self.visual_style),
        ];

        if let Some(desc) = &self.wallpaper_description {
            lines.push(format!("Wallpaper description: {}", desc));
        }

        if let Some(palette) = &self.palette {
            lines.push(format!("Palette: {}", palette));
        }
        if let Some(density) = &self.density {
            lines.push(format!("Density: {}", density));
        }
        if let Some(blur) = &self.blur {
            lines.push(format!("Blur: {}", blur));
        }
        if let Some(transparency) = &self.transparency {
            lines.push(format!("Transparency: {}", transparency));
        }
        if let Some(rounding) = &self.rounding {
            lines.push(format!("Rounding: {}", rounding));
        }

        if !self.stack.is_empty() {
            lines.push(format!(
                "Stack hints: panel={:?}, launcher={:?}, wallpaper={:?}, lockscreen={:?}, notification={:?}",
                self.stack.panel,
                self.stack.launcher,
                self.stack.wallpaper,
                self.stack.lockscreen,
                self.stack.notification
            ));
        }

        if !self.startup_commands.is_empty() {
            lines.push("Startup wiring observed or required:".to_string());
            for command in &self.startup_commands {
                lines.push(format!("- {}", command));
            }
        }

        if !self.ecosystem_components.is_empty() {
            lines.push("Ecosystem components to account for:".to_string());
            for component in &self.ecosystem_components {
                lines.push(format!("- {}", component));
            }
        }

        if !self.completion_notes.is_empty() {
            lines.push("Completeness notes:".to_string());
            for note in &self.completion_notes {
                lines.push(format!("- {}", note));
            }
        }

        if let Some(confidence) = self.confidence {
            lines.push(format!("Confidence: {}%", confidence));
        }

        lines.join("\n")
    }
}

pub fn parse_design_reference_spec(
    raw: &str,
) -> Result<DesignReferenceSpec, Box<dyn std::error::Error + Send + Sync>> {
    let candidate = extract_json_candidate(raw);
    if let Ok(parsed) = serde_json::from_str::<RawDesignReferenceSpec>(&candidate) {
        return Ok(parsed.into_spec());
    }

    let repaired = repair_json_candidate(&candidate);
    if let Ok(parsed) = serde_json::from_str::<RawDesignReferenceSpec>(&repaired) {
        return Ok(parsed.into_spec());
    }

    Err("Failed to parse design reference JSON from model output. The response may have been truncated or contained extra prose.".to_string()
    .into())
}

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
}

fn extract_json_candidate(raw: &str) -> String {
    let trimmed = raw.trim();
    if let Some(fence_start) = trimmed.find("```") {
        let fenced = &trimmed[fence_start + 3..];
        if let Some(fence_end) = fenced.find("```") {
            let mut block = fenced[..fence_end].trim();
            if block.to_lowercase().starts_with("json") {
                block = block[4..].trim_start();
            }
            return block.to_string();
        }
    }

    if let Some(first) = trimmed.find('{') {
        if let Some(last) = trimmed.rfind('}') {
            if last >= first {
                return trimmed[first..=last].trim().to_string();
            }
        }
    }

    trimmed.to_string()
}

fn repair_json_candidate(candidate: &str) -> String {
    let mut cleaned = candidate.trim().to_string();
    loop {
        let next = cleaned.replace(",}", "}").replace(",]", "]");
        if next == cleaned {
            break;
        }
        cleaned = next;
    }

    let mut brace_balance: isize = 0;
    let mut bracket_balance: isize = 0;
    let mut in_string = false;
    let mut escape = false;
    for ch in cleaned.chars() {
        if in_string {
            if escape {
                escape = false;
                continue;
            }
            match ch {
                '\\' => escape = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '{' => brace_balance += 1,
            '}' => brace_balance = (brace_balance - 1).max(0),
            '[' => bracket_balance += 1,
            ']' => bracket_balance = (bracket_balance - 1).max(0),
            _ => {}
        }
    }

    for _ in 0..bracket_balance.max(0) as usize {
        cleaned.push(']');
    }
    for _ in 0..brace_balance.max(0) as usize {
        cleaned.push('}');
    }

    cleaned
}

#[cfg(test)]
mod tests {
    use super::{parse_design_reference_spec, DesignReferenceSpec, DesignStackHints};

    #[test]
    fn applies_stack_hints_over_base_values() {
        let hints = DesignStackHints {
            panel: Some("waybar".to_string()),
            launcher: None,
            wallpaper: Some("hyprpaper".to_string()),
            lockscreen: None,
            notification: Some("swaync".to_string()),
        };

        let (panel, launcher, wallpaper, lockscreen, notification) = hints.apply_to_values(
            "ags".to_string(),
            "rofi".to_string(),
            "none".to_string(),
            "hyprlock".to_string(),
            "dunst".to_string(),
        );

        assert_eq!(panel, "waybar");
        assert_eq!(launcher, "rofi");
        assert_eq!(wallpaper, "hyprpaper");
        assert_eq!(lockscreen, "hyprlock");
        assert_eq!(notification, "swaync");
    }

    #[test]
    fn formats_reference_guidelines() {
        let spec = DesignReferenceSpec {
            summary: "glass setup".to_string(),
            visual_style: "glassmorphic".to_string(),
            palette: Some("Catppuccin Mocha".to_string()),
            density: Some("balanced".to_string()),
            blur: Some("strong".to_string()),
            transparency: Some("high".to_string()),
            rounding: Some("12px".to_string()),
            stack: DesignStackHints {
                panel: Some("waybar".to_string()),
                launcher: Some("rofi".to_string()),
                wallpaper: Some("hyprpaper".to_string()),
                lockscreen: Some("hyprlock".to_string()),
                notification: Some("swaync".to_string()),
            },
            startup_commands: vec!["exec-once = waybar".to_string()],
            ecosystem_components: vec!["hyprpaper".to_string()],
            completion_notes: vec!["wire startup".to_string()],
            confidence: Some(92),
            wallpaper_description: Some("cozy cabin with fireplace in mountains".to_string()),
        };

        let text = spec.to_guidelines();
        assert!(text.contains("REFERENCE DESIGN EXTRACTION"));
        assert!(text.contains("glass setup"));
        assert!(text.contains("waybar"));
        assert!(text.contains("exec-once = waybar"));
    }

    #[test]
    fn normalizes_design_reference_stack_hints() {
        let spec = DesignReferenceSpec {
            summary: "  Glass setup  ".to_string(),
            visual_style: "  Glassmorphic  ".to_string(),
            palette: Some(" Catppuccin ".to_string()),
            density: Some(" balanced ".to_string()),
            blur: Some(" strong ".to_string()),
            transparency: Some(" high ".to_string()),
            rounding: Some(" 12px ".to_string()),
            stack: DesignStackHints {
                panel: Some("WayBar".to_string()),
                launcher: Some("ROFI".to_string()),
                wallpaper: Some(
                    "Hyprpaper2 (or Hyprpaper with custom gradient asset/script/static image matching the pastel gradient background in the screenshot.)"
                        .to_string(),
                ),
                lockscreen: Some("HyprLock".to_string()),
                notification: Some("SwayNC".to_string()),
            },
            startup_commands: vec![" exec-once = waybar ".to_string(), "".to_string()],
            ecosystem_components: vec![" HyprPaper ".to_string(), "hyprpaper".to_string()],
            completion_notes: vec![" wire startup ".to_string()],
            confidence: Some(120),
            wallpaper_description: Some("  misty forest and sunrise  ".to_string()),
        };

        let normalized = spec.normalize();
        assert_eq!(normalized.summary, "Glass setup");
        assert_eq!(normalized.visual_style, "Glassmorphic");
        assert_eq!(normalized.stack.panel.as_deref(), Some("waybar"));
        assert_eq!(normalized.stack.launcher.as_deref(), Some("rofi"));
        assert_eq!(normalized.stack.wallpaper.as_deref(), Some("hyprpaper"));
        assert_eq!(normalized.stack.lockscreen.as_deref(), Some("hyprlock"));
        assert_eq!(normalized.stack.notification.as_deref(), Some("swaync"));
        assert_eq!(normalized.confidence, Some(100));
        assert_eq!(normalized.ecosystem_components, vec!["hyprpaper"]);
    }

    #[test]
    fn parses_reference_spec_from_fenced_json() {
        let raw = r#"
            ```json
            {
              "summary": "glass setup",
              "visual_style": "glassmorphic",
              "stack": {
                "panel": "WayBar"
              },
              "startup_commands": ["waybar"],
              "ecosystem_components": ["hyprpaper"],
              "completion_notes": ["wire startup"],
              "confidence": 87
            }
            ```
        "#;

        let parsed = parse_design_reference_spec(raw).expect("should parse fenced json");
        assert_eq!(parsed.summary, "glass setup");
        assert_eq!(parsed.stack.panel.as_deref(), Some("waybar"));
        assert_eq!(parsed.startup_commands, vec!["waybar"]);
    }

    #[test]
    fn repairs_truncated_reference_json() {
        let raw = r#"{
            "summary": "glass setup",
            "visual_style": "glassmorphic",
            "stack": {
                "panel": "waybar",
                "launcher": "rofi"
            },
            "startup_commands": ["waybar"],
            "ecosystem_components": ["hyprpaper"],
            "completion_notes": ["wire startup"],
            "confidence": 87
        "#;

        let parsed = parse_design_reference_spec(raw).expect("should repair truncated json");
        assert_eq!(parsed.summary, "glass setup");
        assert_eq!(parsed.stack.panel.as_deref(), Some("waybar"));
        assert_eq!(parsed.stack.launcher.as_deref(), Some("rofi"));
    }
}
