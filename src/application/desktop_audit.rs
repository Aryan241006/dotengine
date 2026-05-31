use crate::application::cli_plan::ComponentStack;
use crate::application::ExistingStack;
use crate::domain::SystemContext;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditSeverity {
    Info,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditIssue {
    pub component: String,
    pub severity: AuditSeverity,
    pub detail: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DesktopCompletenessReport {
    pub issues: Vec<AuditIssue>,
}

impl DesktopCompletenessReport {
    pub fn is_clean(&self) -> bool {
        self.issues.is_empty()
    }

    pub fn push(
        &mut self,
        component: impl Into<String>,
        severity: AuditSeverity,
        detail: impl Into<String>,
    ) {
        self.issues.push(AuditIssue {
            component: component.into(),
            severity,
            detail: detail.into(),
        });
    }

    pub fn warning_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|issue| matches!(issue.severity, AuditSeverity::Warning))
            .count()
    }
}

fn selected_components(stack: &ComponentStack) -> [(&str, &str); 5] {
    [
        ("panel", stack.panel.as_str()),
        ("launcher", stack.launcher.as_str()),
        ("wallpaper", stack.wallpaper.as_str()),
        ("lockscreen", stack.lockscreen.as_str()),
        ("notification", stack.notification.as_str()),
    ]
}

fn component_startup_probe(component: &str) -> Option<&'static str> {
    match component {
        "waybar" => Some("waybar"),
        "ags" => Some("ags"),
        "quick-shell" | "quickshell" => Some("quickshell"),
        "hyprpaper" => Some("hyprpaper"),
        "hypridle" => Some("hypridle"),
        "hyprpolkitagent" => Some("hyprpolkitagent"),
        "hyprsunset" => Some("hyprsunset"),
        "swaync" => Some("swaync"),
        "dunst" => Some("dunst"),
        _ => None,
    }
}

fn component_config_label(component: &str) -> &'static str {
    match component {
        "waybar" => ".config/waybar",
        "ags" => ".config/ags",
        "quick-shell" | "quickshell" => ".config/quickshell",
        "rofi" => ".config/rofi",
        "hyprpaper" => ".config/hypr/hyprpaper.conf",
        "hypridle" => ".config/hypr/hypridle.conf",
        "hyprlauncher" => ".config/hyprlauncher",
        "hyprpicker" => ".config/hyprpicker",
        "hyprsunset" => ".config/hyprsunset",
        "hyprpolkitagent" => ".config/hyprpolkitagent",
        "hyprsysteminfo" => ".config/hyprsysteminfo",
        "hyprpwcenter" => ".config/hyprpwcenter",
        "hyprshutdown" => ".config/hyprshutdown",
        "hyprtoolkit" => ".config/hyprtoolkit",
        "hyprland-guiutils" => ".config/hyprland-guiutils",
        "hyprlock" => ".config/hypr/hyprlock.conf",
        "swaylock" => ".config/swaylock/config",
        "waylock" => ".config/waylock/config",
        "swaync" => ".config/swaync",
        "dunst" => ".config/dunst/dunstrc",
        _ => "selected component config",
    }
}

fn component_requires_startup(component: &str) -> bool {
    matches!(
        component,
        "waybar"
            | "ags"
            | "quickshell"
            | "quick-shell"
            | "hyprpaper"
            | "hypridle"
            | "hyprpolkitagent"
            | "hyprsunset"
            | "swaync"
            | "dunst"
    )
}

fn existing_component_for_slot<'a>(
    slot: &str,
    existing_stack: &'a ExistingStack,
) -> Option<&'a str> {
    match slot {
        "panel" => existing_stack.panel.as_deref(),
        "launcher" => existing_stack.launcher.as_deref(),
        "wallpaper" => existing_stack.wallpaper.as_deref(),
        "lockscreen" => existing_stack.lockscreen.as_deref(),
        "notification" => existing_stack.notification.as_deref(),
        _ => None,
    }
}

pub fn audit_desktop_completeness(
    stack: &ComponentStack,
    existing_stack: &ExistingStack,
    system_context: &SystemContext,
    hyprland_config_text: Option<&str>,
) -> DesktopCompletenessReport {
    let mut report = DesktopCompletenessReport::default();

    if system_context
        .package_status
        .get("xdg-desktop-portal-hyprland")
        .copied()
        == Some(false)
    {
        report.push(
            "portal",
            AuditSeverity::Warning,
            "xdg-desktop-portal-hyprland is not detected; screen sharing, file pickers, and sandboxed desktop integration may feel incomplete.",
        );
    }

    if system_context
        .package_status
        .get("hyprpolkitagent")
        .copied()
        == Some(false)
    {
        report.push(
            "polkit",
            AuditSeverity::Warning,
            "hyprpolkitagent is not detected; GUI authentication prompts may be missing from the session.",
        );
    }

    if stack.lockscreen != "none"
        && system_context.package_status.get("hypridle").copied() == Some(false)
    {
        report.push(
            "idle",
            AuditSeverity::Warning,
            "hypridle is not detected; lockscreen-triggering idle behavior may not be wired.",
        );
    }

    if stack.lockscreen != "none"
        && system_context.package_status.get("wlogout").copied() == Some(false)
    {
        report.push(
            "wlogout",
            AuditSeverity::Warning,
            "wlogout is not detected; waybar's power/session overlay button will be unwired or non-functional.",
        );
    }


    for (slot, component) in selected_components(stack) {
        if component == "none" {
            continue;
        }

        let config_label = component_config_label(component);
        let existing_component = existing_component_for_slot(slot, existing_stack);
        let has_existing_config = existing_component.is_some();

        if !has_existing_config {
            report.push(
                component,
                AuditSeverity::Info,
                format!(
                    "No existing {} found; Dotengine will need to create and wire it.",
                    config_label
                ),
            );
        } else if existing_component != Some(component) {
            report.push(
                component,
                AuditSeverity::Warning,
                format!(
                    "Existing {} is configured for '{}' rather than the selected '{}'; the rice may need a replacement or coexistence strategy.",
                    slot,
                    existing_component.unwrap_or("unknown"),
                    component
                ),
            );
        }

        if system_context.package_status.get(component).copied() == Some(false) {
            report.push(
                component,
                AuditSeverity::Warning,
                format!("{} is not detected in PATH; the selected stack may install but not launch cleanly.", component),
            );
        }

        if component_requires_startup(component) {
            let startup_probe = component_startup_probe(component).unwrap_or(component);
            let startup_present = hyprland_config_text
                .map(|contents| contents.contains(startup_probe))
                .unwrap_or(false);
            if !startup_present {
                report.push(
                    component,
                    AuditSeverity::Warning,
                    format!(
                        "No startup wiring for {} was detected in the current Hyprland config; the desktop may feel hollow on login.",
                        component
                    ),
                );
            }
        }
    }

    report
}

#[cfg(test)]
mod tests {
    use super::{audit_desktop_completeness, AuditSeverity};
    use crate::application::cli_plan::ComponentStack;
    use crate::application::ExistingStack;
    use crate::domain::SystemContext;

    #[test]
    fn audits_missing_portal_and_startup_wiring() {
        let mut context = SystemContext::new("TestOS".to_string(), Some("pacman".to_string()));
        context.package_status.insert("waybar".to_string(), true);
        context
            .package_status
            .insert("xdg-desktop-portal-hyprland".to_string(), false);
        context
            .package_status
            .insert("hyprpolkitagent".to_string(), false);
        context.package_status.insert("hypridle".to_string(), false);
        context.package_status.insert("hyprpaper".to_string(), true);

        let stack = ComponentStack::new("waybar", "rofi", "hyprpaper", "hyprlock", "swaync");
        let report =
            audit_desktop_completeness(&stack, &ExistingStack::default(), &context, Some(""));
        assert!(!report.is_clean());
        assert!(report.warning_count() >= 3);
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.component == "portal"
                && matches!(issue.severity, AuditSeverity::Warning)));
    }
}
