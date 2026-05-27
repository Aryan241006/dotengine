use crate::domain::{ConfigFile, ErrorPayload, SystemContext, UserPrompt};
use crate::ports::AiService;
use async_trait::async_trait;
use serde_json::json;

pub struct GeminiClient {
    client: reqwest::Client,
    api_key: String,
}

impl GeminiClient {
    pub fn new(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
        }
    }
}

#[async_trait]
impl AiService for GeminiClient {
    async fn generate_config(
        &self,
        prompt: &UserPrompt,
        system_context: &SystemContext,
        design_rules: &str,
    ) -> Result<Vec<ConfigFile>, Box<dyn std::error::Error + Send + Sync>> {
        let system_instructions = format!(
            "You are an expert Linux Systems and UI/UX Designer specialized in Hyprland, AGS, Rofi, and Waybar.\n\
            Generate a set of premium, visually stunning configuration files that exactly match the user's aesthetic desires.\n\
            \n\
            CRITICAL PATH REQUIREMENT: Every file's 'relative_path' must be relative to the user's home directory and MUST start with '.config/' (e.g. '.config/hypr/hyprland.conf', '.config/waybar/config', '.config/rofi/config.rasi', '.config/dunst/dunstrc'). Any path not starting with '.config/' (e.g., 'hypr/hyprland.conf') will be rejected as a security violation.\n\
            \n\
            You MUST follow the design patterns and rules defined below:\n\
            {}\n\
            \n\
            The user system context is:\n\
            - Operating System: {}\n\
            - Monitors connected: {:?}\n\
            \n\
            Return the output in the schema structure requested.",
            design_rules, system_context.distribution, system_context.monitors
        );

        let user_message_content = if let Some(ref custom) = prompt.custom_guidelines {
            format!(
                "Instructions: {}\nGuidelines: {}",
                prompt.instruction, custom
            )
        } else {
            prompt.instruction.clone()
        };

        // Construct request payload
        let mut parts = vec![
            json!({ "text": format!("{}\n\nUser Prompt: {}", system_instructions, user_message_content) }),
        ];

        for image in &prompt.image_payloads {
            parts.push(json!({
                "inlineData": {
                    "mimeType": image.media_type,
                    "data": image.base64_data
                }
            }));
        }

        let body = json!({
            "contents": [{ "parts": parts }],
            "generationConfig": {
                "responseMimeType": "application/json",
                "responseSchema": {
                    "type": "OBJECT",
                    "properties": {
                        "configs": {
                            "type": "ARRAY",
                            "items": {
                                "type": "OBJECT",
                                "properties": {
                                    "relative_path": { "type": "STRING" },
                                    "content": { "type": "STRING" }
                                },
                                "required": ["relative_path", "content"]
                            }
                        }
                    },
                    "required": ["configs"]
                },
                "temperature": 0.2
            }
        });

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-3.5-flash:generateContent?key={}",
            self.api_key
        );

        let response = self.client.post(&url).json(&body).send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Gemini API error: {}", error_text).into());
        }

        let resp_json: serde_json::Value = response.json().await?;
        let content_str = resp_json["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .ok_or_else(|| "Failed to extract text from Gemini candidate")?;

        let parsed: serde_json::Value = serde_json::from_str(content_str)?;
        let configs_value = parsed["configs"]
            .as_array()
            .ok_or_else(|| "Generated JSON from Gemini lacks a 'configs' array")?;

        let mut configs = Vec::new();
        for val in configs_value {
            let relative_path = val["relative_path"]
                .as_str()
                .ok_or_else(|| "Config file lacks 'relative_path'")?
                .to_string();
            let content = val["content"]
                .as_str()
                .ok_or_else(|| "Config file lacks 'content'")?
                .to_string();
            configs.push(ConfigFile::new(relative_path, content));
        }

        Ok(configs)
    }

    async fn repair_config(
        &self,
        error_payload: &ErrorPayload,
        design_rules: &str,
    ) -> Result<Vec<ConfigFile>, Box<dyn std::error::Error + Send + Sync>> {
        let system_instructions = format!(
            "You are an expert self-healing agent specializing in reloading and syntax errors of Hyprland, Rofi, and AGS.\n\
            You are given a list of configuration files that were generated and the resulting error diagnostics when attempting to apply them.\n\
            \n\
            CRITICAL PATH REQUIREMENT: Every file's 'relative_path' must be relative to the user's home directory and MUST start with '.config/' (e.g. '.config/hypr/hyprland.conf', '.config/waybar/config', '.config/rofi/config.rasi', '.config/dunst/dunstrc'). Any path not starting with '.config/' (e.g., 'hypr/hyprland.conf') will be rejected as a security violation.\n\
            \n\
            Analyze the error logs carefully, apply corrections to the configurations, and output the patched configurations.\n\
            Do NOT change aesthetic design choices unless it directly impacts the syntax error.\n\
            \n\
            Design principles context:\n\
            {}\n\
            \n\
            Respond strictly in the requested JSON schema format.",
            design_rules
        );

        let user_content = json!({
            "command": error_payload.command,
            "exit_code": error_payload.exit_code,
            "stdout": error_payload.stdout,
            "stderr": error_payload.stderr,
            "failing_configs": error_payload.active_configs
        });

        let parts = vec![
            json!({ "text": format!("{}\n\nSystem Error Context:\n{}", system_instructions, serde_json::to_string_pretty(&user_content)?) }),
        ];

        let body = json!({
            "contents": [{ "parts": parts }],
            "generationConfig": {
                "responseMimeType": "application/json",
                "responseSchema": {
                    "type": "OBJECT",
                    "properties": {
                        "configs": {
                            "type": "ARRAY",
                            "items": {
                                "type": "OBJECT",
                                "properties": {
                                    "relative_path": { "type": "STRING" },
                                    "content": { "type": "STRING" }
                                },
                                "required": ["relative_path", "content"]
                            }
                        }
                    },
                    "required": ["configs"]
                },
                "temperature": 0.1
            }
        });

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-3.5-flash:generateContent?key={}",
            self.api_key
        );

        let response = self.client.post(&url).json(&body).send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Gemini API repair error: {}", error_text).into());
        }

        let resp_json: serde_json::Value = response.json().await?;
        let content_str = resp_json["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .ok_or_else(|| "Failed to extract text from Gemini repair candidate")?;

        let parsed: serde_json::Value = serde_json::from_str(content_str)?;
        let configs_value = parsed["configs"]
            .as_array()
            .ok_or_else(|| "Repaired JSON from Gemini lacks a 'configs' array")?;

        let mut repaired_configs = Vec::new();
        for val in configs_value {
            let relative_path = val["relative_path"]
                .as_str()
                .ok_or_else(|| "Config file lacks 'relative_path'")?
                .to_string();
            let content = val["content"]
                .as_str()
                .ok_or_else(|| "Config file lacks 'content'")?
                .to_string();
            repaired_configs.push(ConfigFile::new(relative_path, content));
        }

        Ok(repaired_configs)
    }
}
