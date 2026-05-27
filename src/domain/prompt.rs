use base64::Engine;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImagePayload {
    pub base64_data: String,
    pub media_type: String,
}

impl ImagePayload {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        let media_type = if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            "image/png"
        } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
            "image/jpeg"
        } else {
            return Err("Unsupported image format. Provide a PNG or JPEG image.".to_string());
        };

        Ok(Self {
            base64_data: base64::engine::general_purpose::STANDARD.encode(bytes),
            media_type: media_type.to_string(),
        })
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserPrompt {
    /// The core natural language instruction describing the desired desktop aesthetic
    pub instruction: String,

    /// Optional multi-modal inputs preserving encoded bytes and detected media types.
    pub image_payloads: Vec<ImagePayload>,

    /// User selected theme styling rules or parameters that supplement the prompt
    pub custom_guidelines: Option<String>,
}

impl UserPrompt {
    pub fn new(instruction: String) -> Self {
        Self {
            instruction,
            image_payloads: Vec::new(),
            custom_guidelines: None,
        }
    }

    pub fn with_images(mut self, images: Vec<ImagePayload>) -> Self {
        self.image_payloads = images;
        self
    }

    pub fn with_guidelines(mut self, guidelines: String) -> Self {
        self.custom_guidelines = Some(guidelines);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::{ImagePayload, UserPrompt};

    #[test]
    fn detects_png_image_payloads() {
        let payload = ImagePayload::from_bytes(b"\x89PNG\r\n\x1a\nbody").unwrap();
        assert_eq!(payload.media_type, "image/png");
    }

    #[test]
    fn detects_jpeg_image_payloads() {
        let payload = ImagePayload::from_bytes(&[0xff, 0xd8, 0xff, 0xe0]).unwrap();
        assert_eq!(payload.media_type, "image/jpeg");
    }

    #[test]
    fn rejects_unsupported_image_payloads() {
        assert!(ImagePayload::from_bytes(b"GIF89a").is_err());
    }

    #[test]
    fn retains_multiple_image_attachments() {
        let prompt = UserPrompt::new("match screenshots".to_string()).with_images(vec![
            ImagePayload::from_bytes(b"\x89PNG\r\n\x1a\nfirst").unwrap(),
            ImagePayload::from_bytes(&[0xff, 0xd8, 0xff, 0xe0]).unwrap(),
        ]);

        assert_eq!(prompt.image_payloads.len(), 2);
        assert_eq!(prompt.image_payloads[0].media_type, "image/png");
        assert_eq!(prompt.image_payloads[1].media_type, "image/jpeg");
    }
}
