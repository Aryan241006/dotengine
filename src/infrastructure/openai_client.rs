use crate::domain::{
    design_reference::parse_design_reference_spec, ConfigFile, DesignReferenceSpec, ErrorPayload,
    SystemContext, UserPrompt,
};
use crate::ports::AiService;
use async_trait::async_trait;
use serde_json::json;

pub struct OpenaiClient {
    client: reqwest::Client,
    api_key: String,
}

impl OpenaiClient {
    pub fn new(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
        }
    }
}

#[async_trait]
impl AiService for OpenaiClient {
    async fn analyze_design_reference(
        &self,
        prompt: &UserPrompt,
        system_context: &SystemContext,
        design_rules: &str,
    ) -> Result<DesignReferenceSpec, Box<dyn std::error::Error + Send + Sync>> {
        let system_instructions = format!(
            "You are an expert Hyprland visual analyst.\n\
            Infer the visible design language from the screenshot(s) and produce a compact JSON manifest for later desktop generation.\n\
            Return only JSON.\n\
            \n\
            Required fields: summary, visual_style, stack, startup_commands, ecosystem_components, completion_notes, confidence, wallpaper_description.\n\
            Optional fields: palette, density, blur, transparency, rounding.\n\
            Keep component names canonical and lowercase. If unsure, use null.\n\
            \n\
            CRITICAL: Analyze the visible desktop wallpaper in the screenshot(s) and write a highly precise, search-friendly descriptive query for it in 'wallpaper_description' (e.g., 'minimalist forest pine trees mist' or 'retro cyberpunk neon street rain').\n\
            \n\
            The user system context is:\n\
            - Operating System: {}\n\
            - Monitors connected: {:?}\n\
            \n\
            Reference-analysis brief:\n\
            {}",
            system_context.distribution, system_context.monitors, design_rules
        );

        let user_message_content = if let Some(ref custom) = prompt.custom_guidelines {
            format!(
                "Reference analysis request.\nPrompt context: {}\nAdditional guidance: {}",
                prompt.instruction, custom
            )
        } else {
            format!(
                "Reference analysis request.\nPrompt context: {}",
                prompt.instruction
            )
        };

        let mut messages = vec![json!({
            "role": "system",
            "content": system_instructions
        })];

        if !prompt.image_payloads.is_empty() {
            let mut content = vec![json!({ "type": "text", "text": user_message_content })];
            for image in &prompt.image_payloads {
                content.push(json!({
                    "type": "image_url",
                    "image_url": {
                        "url": format!("data:{};base64,{}", image.media_type, image.base64_data),
                        "detail": "high"
                    }
                }));
            }
            messages.push(json!({
                "role": "user",
                "content": content
            }));
        } else {
            messages.push(json!({
                "role": "user",
                "content": user_message_content
            }));
        }

        let body = json!({
            "model": "gpt-4o",
            "messages": messages,
            "response_format": { "type": "json_object" },
            "temperature": 0.2,
            "max_tokens": 2048
        });

        let response = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("OpenAI API reference-analysis error: {}", error_text).into());
        }

        let resp_json: serde_json::Value = response.json().await?;
        let content_str = resp_json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or("Failed to extract content from OpenAI reference analysis")?;
        parse_design_reference_spec(content_str)
    }

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
            Respond ONLY in the following JSON format. Do NOT wrap the JSON in markdown code blocks. Do NOT include any conversation or explanation.\n\
            JSON Structure:\n\
            {{\n\
              \"configs\": [\n\
                {{\n\
                  \"relative_path\": \".config/hypr/hyprland.conf\",\n\
                  \"content\": \"file contents here\"\n\
                }}\n\
              ]\n\
            }}",
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
        let mut messages = vec![json!({
            "role": "system",
            "content": system_instructions
        })];

        if !prompt.image_payloads.is_empty() {
            let mut content = vec![json!({ "type": "text", "text": user_message_content })];
            for image in &prompt.image_payloads {
                content.push(json!({
                    "type": "image_url",
                    "image_url": {
                        "url": format!("data:{};base64,{}", image.media_type, image.base64_data)
                    }
                }));
            }
            messages.push(json!({
                "role": "user",
                "content": content
            }));
        } else {
            messages.push(json!({
                "role": "user",
                "content": user_message_content
            }));
        }

        let body = json!({
            "model": "gpt-4o",
            "messages": messages,
            "response_format": { "type": "json_object" },
            "temperature": 0.2
        });

        let response = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("OpenAI API error: {}", error_text).into());
        }

        let resp_json: serde_json::Value = response.json().await?;
        let content_str = resp_json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or("Failed to extract content from OpenAI response")?;

        let parsed: serde_json::Value = serde_json::from_str(content_str)?;
        let configs_value = parsed["configs"]
            .as_array()
            .ok_or("Generated JSON lacks a 'configs' array")?;

        let mut configs = Vec::new();
        for val in configs_value {
            let relative_path = val["relative_path"]
                .as_str()
                .ok_or("Config file lacks 'relative_path'")?
                .to_string();
            let content = val["content"]
                .as_str()
                .ok_or("Config file lacks 'content'")?
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
            Respond ONLY in the following JSON format. Do NOT wrap in markdown, do NOT include conversational text.\n\
            {{\n\
              \"configs\": [\n\
                {{\n\
                  \"relative_path\": \".config/hypr/hyprland.conf\",\n\
                  \"content\": \"file contents here\"\n\
                }}\n\
              ]\n\
            }}",
            design_rules
        );

        let user_content = json!({
            "command": error_payload.command,
            "exit_code": error_payload.exit_code,
            "stdout": error_payload.stdout,
            "stderr": error_payload.stderr,
            "failing_configs": error_payload.active_configs
        });

        let body = json!({
            "model": "gpt-4o",
            "messages": [
                { "role": "system", "content": system_instructions },
                { "role": "user", "content": serde_json::to_string_pretty(&user_content)? }
            ],
            "response_format": { "type": "json_object" },
            "temperature": 0.1
        });

        let response = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("OpenAI API repair request error: {}", error_text).into());
        }

        let resp_json: serde_json::Value = response.json().await?;
        let content_str = resp_json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or("Failed to extract content from OpenAI repair response")?;

        let parsed: serde_json::Value = serde_json::from_str(content_str)?;
        let configs_value = parsed["configs"]
            .as_array()
            .ok_or("Repaired JSON lacks a 'configs' array")?;

        let mut repaired_configs = Vec::new();
        for val in configs_value {
            let relative_path = val["relative_path"]
                .as_str()
                .ok_or("Config file lacks 'relative_path'")?
                .to_string();
            let content = val["content"]
                .as_str()
                .ok_or("Config file lacks 'content'")?
                .to_string();
            repaired_configs.push(ConfigFile::new(relative_path, content));
        }

        Ok(repaired_configs)
    }
}
