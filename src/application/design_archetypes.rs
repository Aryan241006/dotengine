#[derive(Debug)]
pub struct DesignArchetype {
    pub name: &'static str,
    pub keywords: &'static [&'static str],
    pub recommended_stack: &'static [&'static str],
    pub summary: &'static str,
    pub guidance: &'static str,
}

const ARCHETYPES: &[DesignArchetype] = &[
    DesignArchetype {
        name: "Glassmorphic Glow",
        keywords: &["glass", "blur", "glow", "frost", "frosted", "widget"],
        recommended_stack: &["ags", "rofi", "hyprpaper"],
        summary: "Vibrant frosted-glass overlays, unified dashboard widgets, and layered translucency.",
        guidance:
            "Prioritize blur, rounded cards, atmospheric shadows, and startup wiring for the shell and wallpaper flow.",
    },
    DesignArchetype {
        name: "Nordic Minimalist",
        keywords: &["minimal", "nord", "arctic", "clean", "lightweight"],
        recommended_stack: &["waybar", "dunst", "rofi", "hyprpaper"],
        summary: "Clean Arctic layouts with compact modules and low overhead.",
        guidance:
            "Favor sparse layout density, crisp contrast, and direct startup wiring for panel, notification, and wallpaper components.",
    },
    DesignArchetype {
        name: "Retro Cyberpunk",
        keywords: &["cyber", "neon", "retro", "tokyo", "night", "magenta"],
        recommended_stack: &["quick-shell", "rofi", "dunst", "hyprpaper"],
        summary: "High-contrast neon surfaces with a fast, layered, futuristic feel.",
        guidance:
            "Prefer vivid accents, aggressive contrast, and fluid overlays without sacrificing startup completeness.",
    },
    DesignArchetype {
        name: "Terminal-First Functional",
        keywords: &["terminal", "functional", "sparse", "simple", "bare"],
        recommended_stack: &["waybar", "rofi", "hyprpaper"],
        summary: "Sparse, legible, and focused on speed over decoration.",
        guidance:
            "Keep the surface minimal, but still wire wallpaper, launcher, and panel behavior so the desktop feels complete.",
    },
];

pub fn infer_archetype(
    prompt: &str,
    template_name: Option<&str>,
) -> Option<&'static DesignArchetype> {
    if let Some(name) = template_name {
        let lower = name.to_lowercase();
        if let Some(profile) = ARCHETYPES.iter().find(|profile| {
            lower.contains(&profile.name.to_lowercase())
                || profile
                    .keywords
                    .iter()
                    .any(|kw| lower.contains(kw) || name.to_lowercase().contains(kw))
        }) {
            return Some(profile);
        }
    }

    let lower = prompt.to_lowercase();
    ARCHETYPES.iter().find(|profile| {
        profile
            .keywords
            .iter()
            .any(|kw| lower.contains(kw) || lower.contains(&profile.name.to_lowercase()))
    })
}

pub fn archetype_guidelines(prompt: &str, template_name: Option<&str>) -> Option<String> {
    infer_archetype(prompt, template_name).map(|profile| {
        format!(
            "ARCHETYPE PROFILE: {}\n{}\nRECOMMENDED STACK SIGNAL: {}",
            profile.name,
            profile.guidance,
            profile.recommended_stack.join(", ")
        )
    })
}

pub fn archetype_recommendation_lines(
    prompt: &str,
    template_name: Option<&str>,
) -> Option<Vec<String>> {
    infer_archetype(prompt, template_name).map(|profile| {
        let mut lines = vec![format!("Target Aesthetic: {}", profile.summary)];
        for component in profile.recommended_stack {
            lines.push(format!("- [RECOMMENDED] {}", component));
        }
        lines
    })
}

#[cfg(test)]
mod tests {
    use super::{archetype_guidelines, infer_archetype};

    #[test]
    fn infers_glassmorphic_archetype_from_prompt() {
        let profile = infer_archetype("bright glass blur desktop", None).expect("profile");
        assert_eq!(profile.name, "Glassmorphic Glow");
    }

    #[test]
    fn emits_archetype_guidelines_for_known_template() {
        let guidelines = archetype_guidelines("minimal workbench", Some("Nordic Minimalist"))
            .expect("guidelines");
        assert!(guidelines.contains("Nordic Minimalist"));
        assert!(guidelines.contains("RECOMMENDED STACK SIGNAL"));
    }
}
