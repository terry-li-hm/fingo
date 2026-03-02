use std::fmt;

#[derive(Debug)]
pub enum AppError {
    ApiKeyMissing,
    #[allow(dead_code)]
    Silent,
    Message(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ApiKeyMissing => {
                write!(f, "Error: Gemini API key not found. Run: fingo key save <key>")
            }
            Self::Silent => Ok(()),
            Self::Message(msg) => write!(f, "{msg}"),
        }
    }
}
