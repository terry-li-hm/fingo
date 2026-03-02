use std::time::Duration;

use serde::Deserialize;

use crate::error::AppError;

const GEMINI_MODELS_URL: &str = "https://generativelanguage.googleapis.com/v1beta/models";

#[derive(Debug, Deserialize)]
struct ModelsResponse {
    models: Option<Vec<Model>>,
}

#[derive(Debug, Deserialize)]
struct Model {
    name: Option<String>,
    #[serde(rename = "displayName")]
    display_name: Option<String>,
    description: Option<String>,
    #[serde(rename = "supportedGenerationMethods")]
    supported_generation_methods: Option<Vec<String>>,
}

pub fn list_image_models(api_key: &str) -> Result<(), AppError> {
    let config = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(60)))
        .build();
    let agent: ureq::Agent = config.into();

    let response = agent
        .get(GEMINI_MODELS_URL)
        .header("x-goog-api-key", api_key)
        .call()
        .map_err(|err| AppError::Message(format!("Error: {err}")))?;

    let mut body = response.into_body();
    let parsed: ModelsResponse = body
        .read_json()
        .map_err(|err| AppError::Message(format!("Error: {err}")))?;

    let mut printed = 0usize;

    if let Some(models) = parsed.models {
        for model in models {
            if is_image_capable(&model) {
                let name = model.name.unwrap_or_else(|| "(unknown-name)".to_string());
                let display = model
                    .display_name
                    .unwrap_or_else(|| "(unknown-display-name)".to_string());
                println!("{name} - {display}");
                printed += 1;
            }
        }
    }

    if printed == 0 {
        println!("No image-capable models found.");
    }

    Ok(())
}

fn is_image_capable(model: &Model) -> bool {
    let name = model
        .name
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let display = model
        .display_name
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let description = model
        .description
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();

    if name.contains("image")
        || name.contains("banana")
        || display.contains("image")
        || display.contains("banana")
        || description.contains("image")
        || description.contains("edit")
    {
        return true;
    }

    model
        .supported_generation_methods
        .as_ref()
        .map(|methods| {
            methods.iter().any(|m| {
                matches!(
                    m.as_str(),
                    "generateImages" | "editImage" | "generateContent"
                )
            }) && (name.contains("gemini-2.0-flash-exp") || name.contains("nano-banana"))
        })
        .unwrap_or(false)
}
