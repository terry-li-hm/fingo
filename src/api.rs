use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};

use crate::error::AppError;

const GEMINI_API_BASE: &str = "https://generativelanguage.googleapis.com/v1beta/models";

#[derive(Debug)]
pub struct GeneratedImage {
    pub bytes: Vec<u8>,
    pub mime_type: String,
}

#[derive(Debug, Serialize)]
struct GenerateRequest {
    contents: Vec<Content>,
    #[serde(rename = "generationConfig")]
    generation_config: GenerationConfig,
}

#[derive(Debug, Serialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum Part {
    Text {
        text: String,
    },
    #[serde(rename_all = "snake_case")]
    InlineData {
        inline_data: InlineData,
    },
}

#[derive(Debug, Serialize)]
struct InlineData {
    mime_type: String,
    data: String,
}

#[derive(Debug, Serialize)]
struct GenerationConfig {
    #[serde(rename = "responseModalities")]
    response_modalities: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct GenerateResponse {
    candidates: Option<Vec<Candidate>>,
}

#[derive(Debug, Deserialize)]
struct Candidate {
    #[serde(rename = "finishReason")]
    finish_reason: Option<String>,
    content: Option<ResponseContent>,
}

#[derive(Debug, Deserialize)]
struct ResponseContent {
    parts: Option<Vec<ResponsePart>>,
}

#[derive(Debug, Deserialize)]
struct ResponsePart {
    #[serde(rename = "inlineData", alias = "inline_data")]
    inline_data: Option<ResponseInlineData>,
}

#[derive(Debug, Deserialize)]
struct ResponseInlineData {
    #[serde(rename = "mimeType")]
    #[serde(alias = "mime_type")]
    mime_type: Option<String>,
    data: Option<String>,
}

pub fn generate_image(model: &str, key: &str, prompt: &str) -> Result<GeneratedImage, AppError> {
    generate_with_optional_image(model, key, prompt, None)
}

pub fn edit_image(
    model: &str,
    key: &str,
    prompt: &str,
    image_path: &Path,
) -> Result<GeneratedImage, AppError> {
    generate_with_optional_image(model, key, prompt, Some(image_path))
}

fn generate_with_optional_image(
    model: &str,
    key: &str,
    prompt: &str,
    image_path: Option<&Path>,
) -> Result<GeneratedImage, AppError> {
    let mut parts = vec![Part::Text {
        text: prompt.to_string(),
    }];

    if let Some(path) = image_path {
        let image_bytes = fs::read(path)
            .map_err(|err| AppError::Message(format!("Error: failed to read image: {err}")))?;
        let encoded = STANDARD.encode(image_bytes);
        let input_mime_type = detect_input_mime(path);
        parts.push(Part::InlineData {
            inline_data: InlineData {
                mime_type: input_mime_type.to_string(),
                data: encoded,
            },
        });
    }

    let payload = GenerateRequest {
        contents: vec![Content { parts }],
        generation_config: GenerationConfig {
            response_modalities: vec!["image".to_string(), "text".to_string()],
        },
    };

    let url = format!("{GEMINI_API_BASE}/{model}:generateContent");

    let config = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(60)))
        .build();
    let agent: ureq::Agent = config.into();

    let response = agent
        .post(&url)
        .header("x-goog-api-key", key)
        .send_json(&payload)
        .map_err(|err| AppError::Message(format!("Error: {err}")))?;

    let mut body = response.into_body();
    let parsed: GenerateResponse = body
        .read_json()
        .map_err(|err| AppError::Message(format!("Error: {err}")))?;

    extract_generated_image(parsed)
}

fn extract_generated_image(response: GenerateResponse) -> Result<GeneratedImage, AppError> {
    let candidates = response
        .candidates
        .ok_or_else(|| AppError::Message("Error: API response missing candidates".to_string()))?;

    let candidate = candidates
        .first()
        .ok_or_else(|| AppError::Message("Error: API response had no candidates".to_string()))?;

    if let Some(reason) = candidate.finish_reason.as_deref()
        && reason != "STOP"
        && reason != "MAX_TOKENS"
    {
        return Err(AppError::Message(format!(
            "Error: generation blocked with finishReason={reason}"
        )));
    }

    let content = candidate.content.as_ref().ok_or_else(|| {
        AppError::Message("Error: API response missing candidate content".to_string())
    })?;

    let parts = content.parts.as_ref().ok_or_else(|| {
        AppError::Message("Error: API response missing content parts".to_string())
    })?;

    for part in parts {
        if let Some(inline_data) = part.inline_data.as_ref() {
            let encoded = inline_data.data.as_ref().ok_or_else(|| {
                AppError::Message("Error: inlineData.data missing in image response".to_string())
            })?;

            let bytes = STANDARD
                .decode(encoded)
                .map_err(|err| AppError::Message(format!("Error: invalid base64 image: {err}")))?;

            let mime_type = inline_data
                .mime_type
                .clone()
                .unwrap_or_else(|| "image/jpeg".to_string());

            return Ok(GeneratedImage { bytes, mime_type });
        }
    }

    Err(AppError::Message(
        "Error: no image data returned by API".to_string(),
    ))
}

pub fn output_path_for_mime(base: &Path, mime_type: &str) -> PathBuf {
    let ext = match mime_type {
        "image/png" => "png",
        "image/webp" => "webp",
        "image/jpg" | "image/jpeg" => "jpg",
        _ => "jpg",
    };

    let mut path = base.to_path_buf();
    path.set_extension(ext);
    path
}

fn detect_input_mime(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "png" => "image/png",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "jpg" | "jpeg" => "image/jpeg",
        _ => "image/jpeg",
    }
}
