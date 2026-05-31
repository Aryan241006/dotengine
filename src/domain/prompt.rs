use base64::Engine;
use std::io::Cursor;
use image::{imageops::FilterType, DynamicImage, GenericImageView, ImageOutputFormat};

const REFERENCE_IMAGE_MAX_DIMENSION: u32 = 1024;
const REFERENCE_IMAGE_TARGET_BYTES: usize = 500 * 1024; // 500 KB target for vision speed

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImagePayload {
    pub base64_data: String,
    pub media_type: String,
}

impl ImagePayload {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        let media_type = detect_media_type(bytes)?;
        Ok(Self::new(bytes, media_type))
    }

    /// Preprocesses any image input (PNG, JPEG, BMP, GIF) by decoding it natively,
    /// resizing it with a high-fidelity CatmullRom filter, and compressing it as a high-density,
    /// low-latency JPEG payload optimized for modern LLM vision models.
    pub fn from_reference_image_bytes(bytes: &[u8]) -> Result<Self, String> {
        let decoded = image::load_from_memory(bytes)
            .map_err(|e| format!("Failed to decode image input: {}", e))?;

        let optimized = optimize_image_for_reference(&decoded)?;
        Ok(Self::new(&optimized, "image/jpeg"))
    }

    fn new(bytes: &[u8], media_type: &str) -> Self {
        Self {
            base64_data: base64::engine::general_purpose::STANDARD.encode(bytes),
            media_type: media_type.to_string(),
        }
    }
}

fn detect_media_type(bytes: &[u8]) -> Result<&'static str, String> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Ok("image/png")
    } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        Ok("image/jpeg")
    } else if bytes.starts_with(b"GIF8") {
        Ok("image/gif")
    } else if bytes.starts_with(b"BM") {
        Ok("image/bmp")
    } else {
        Err("Unsupported image format. Provide a PNG, JPEG, GIF, or BMP image.".to_string())
    }
}

fn optimize_image_for_reference(img: &DynamicImage) -> Result<Vec<u8>, String> {
    let (width, height) = img.dimensions();

    // Scale to vision-friendly resolution candidates
    let max_dimension = REFERENCE_IMAGE_MAX_DIMENSION;
    let resized = if width > max_dimension || height > max_dimension {
        // CatmullRom offers high fidelity downscaling, preserving thin UI lines and text perfectly
        img.resize(max_dimension, max_dimension, FilterType::CatmullRom)
    } else {
        img.clone()
    };

    // Try encoding to JPEG at standard quality settings (85, 75)
    for quality in [85_u8, 75] {
        let mut buffer = Cursor::new(Vec::new());
        resized
            .write_to(&mut buffer, ImageOutputFormat::Jpeg(quality))
            .map_err(|e| format!("Failed to encode optimized JPEG: {}", e))?;

        let bytes = buffer.into_inner();
        if bytes.len() <= REFERENCE_IMAGE_TARGET_BYTES {
            return Ok(bytes);
        }
    }

    // Fallback settings if still too large
    let mut buffer = Cursor::new(Vec::new());
    resized
        .write_to(&mut buffer, ImageOutputFormat::Jpeg(60))
        .map_err(|e| format!("Failed to encode fallback JPEG: {}", e))?;
    Ok(buffer.into_inner())
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
    use image::ImageOutputFormat;
    use std::io::Cursor;

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
        assert!(ImagePayload::from_bytes(b"not an image at all").is_err());
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

    #[test]
    fn preprocesses_reference_pngs() {
        let mut raw = Vec::new();
        {
            let img = image::DynamicImage::new_rgba8(4, 4);
            let mut cursor = Cursor::new(&mut raw);
            img.write_to(&mut cursor, ImageOutputFormat::Png).unwrap();
        }

        let payload = ImagePayload::from_reference_image_bytes(&raw).unwrap();
        assert_eq!(payload.media_type, "image/jpeg");
        assert!(!payload.base64_data.is_empty());
    }

    #[test]
    fn preprocesses_reference_jpegs() {
        let mut raw = Vec::new();
        {
            let img = image::DynamicImage::new_rgb8(4, 4);
            let mut cursor = Cursor::new(&mut raw);
            img.write_to(&mut cursor, ImageOutputFormat::Jpeg(90)).unwrap();
        }

        let payload = ImagePayload::from_reference_image_bytes(&raw).unwrap();
        assert_eq!(payload.media_type, "image/jpeg");
        assert!(!payload.base64_data.is_empty());
    }
}
