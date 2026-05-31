pub fn normalize_component_value(slot: &str, raw: &str) -> Option<String> {
    let lower = raw.trim().to_lowercase();
    if lower.is_empty() {
        return None;
    }

    let normalized = match slot {
        "panel" => match_component(
            &lower,
            &["waybar", "ags", "quickshell", "quick-shell", "none"],
            &[
                ("waybar", &["waybar"][..]),
                ("ags", &["ags", "aylur", "aylurs"][..]),
                ("quick-shell", &["quickshell", "quick-shell"][..]),
                ("none", &["no panel", "without panel", "none"][..]),
            ],
        ),
        "launcher" => match_component(
            &lower,
            &["rofi", "none"],
            &[
                ("rofi", &["rofi"][..]),
                ("none", &["no launcher", "without launcher", "none"][..]),
            ],
        ),
        "wallpaper" => match_component(
            &lower,
            &["hyprpaper", "none"],
            &[
                (
                    "hyprpaper",
                    &["hyprpaper", "swww", "wallpaper", "background"][..],
                ),
                ("none", &["no wallpaper", "without wallpaper", "none"][..]),
            ],
        ),
        "lockscreen" => match_component(
            &lower,
            &["hyprlock", "swaylock", "waylock", "none"],
            &[
                ("hyprlock", &["hyprlock"][..]),
                ("swaylock", &["swaylock"][..]),
                ("waylock", &["waylock"][..]),
                ("none", &["no lockscreen", "without lockscreen", "none"][..]),
            ],
        ),
        "notification" => match_component(
            &lower,
            &["swaync", "dunst", "none"],
            &[
                ("swaync", &["swaync"][..]),
                ("dunst", &["dunst"][..]),
                (
                    "none",
                    &["no notification", "without notifications", "none"][..],
                ),
            ],
        ),
        _ => return None,
    };

    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

pub fn normalize_stack_values(
    panel: String,
    launcher: String,
    wallpaper: String,
    lockscreen: String,
    notification: String,
) -> (String, String, String, String, String) {
    (
        normalize_component_value("panel", &panel).unwrap_or_default(),
        normalize_component_value("launcher", &launcher).unwrap_or_default(),
        normalize_component_value("wallpaper", &wallpaper).unwrap_or_default(),
        normalize_component_value("lockscreen", &lockscreen).unwrap_or_default(),
        normalize_component_value("notification", &notification).unwrap_or_default(),
    )
}

fn match_component(lower: &str, allowed: &[&str], aliases: &[(&str, &[&str])]) -> String {
    if allowed.contains(&lower) {
        return lower.to_string();
    }

    for (canonical, keywords) in aliases {
        if keywords.iter().any(|keyword| lower.contains(keyword)) {
            return (*canonical).to_string();
        }
    }

    String::new()
}

#[cfg(test)]
mod tests {
    use super::{normalize_component_value, normalize_stack_values};

    #[test]
    fn normalizes_prose_to_supported_wallpaper_component() {
        let value = normalize_component_value(
            "wallpaper",
            "Hyprpaper2 (or Hyprpaper with custom gradient asset/script/static image matching the pastel gradient background in the screenshot.)",
        )
        .expect("canonical component");
        assert_eq!(value, "hyprpaper");
    }

    #[test]
    fn normalizes_case_and_aliases() {
        let (panel, launcher, wallpaper, lockscreen, notification) = normalize_stack_values(
            "WayBar".to_string(),
            "ROFI".to_string(),
            "swww".to_string(),
            "HyprLock".to_string(),
            "SwayNC".to_string(),
        );

        assert_eq!(panel, "waybar");
        assert_eq!(launcher, "rofi");
        assert_eq!(wallpaper, "hyprpaper");
        assert_eq!(lockscreen, "hyprlock");
        assert_eq!(notification, "swaync");
    }
}
