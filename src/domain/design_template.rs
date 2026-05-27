#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DesignTemplate {
    pub name: String,
    pub description: String,
    pub recommended_colors: String,
    /// The software combo recommended for this template (e.g. vec!["ags", "rofi"])
    pub recommended_stack: Vec<String>,
    pub recommended_rounding: u32,
    pub recommended_gaps_in: u32,
    pub recommended_gaps_out: u32,
    pub dynamic_animations: bool,
}

impl DesignTemplate {
    /// Returns the built-in library of top-tier desktop profiles
    pub fn get_predefined_library() -> Vec<Self> {
        vec![
            Self {
                name: "Glassmorphic Glow".to_string(),
                description: "Vibrant frosted-glass overlays, unified dashboard widgets, and organic animations. Best suited for modern setups.".to_string(),
                recommended_colors: "Catppuccin Mocha (Mauve/Blue accents, transparent mantle)".to_string(),
                recommended_stack: vec!["ags".to_string(), "rofi".to_string(), "hyprpaper".to_string()],
                recommended_rounding: 12,
                recommended_gaps_in: 6,
                recommended_gaps_out: 12,
                dynamic_animations: true,
            },
            Self {
                name: "Nordic Minimalist".to_string(),
                description: "Super clean Arctic-themed workspaces, high usability, light resource footprint, with rigid layouts.".to_string(),
                recommended_colors: "Nord Palette (Frost highlights with slate background)".to_string(),
                recommended_stack: vec!["waybar".to_string(), "dunst".to_string(), "rofi".to_string(), "hyprpaper".to_string()],
                recommended_rounding: 6,
                recommended_gaps_in: 4,
                recommended_gaps_out: 8,
                dynamic_animations: false,
            },
            Self {
                name: "Retro Cyberpunk".to_string(),
                description: "Neon magenta accents, high contrast indicators, fluid and swift layout scaling.".to_string(),
                recommended_colors: "Tokyo Night Palette (Hot pink, cyan, dark terminal base)".to_string(),
                recommended_stack: vec!["quick-shell".to_string(), "rofi".to_string(), "dunst".to_string(), "hyprpaper".to_string()],
                recommended_rounding: 0, // angular cyberpunk cuts
                recommended_gaps_in: 8,
                recommended_gaps_out: 15,
                dynamic_animations: true,
            },
        ]
    }
}
