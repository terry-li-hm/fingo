mod api;
mod error;
mod keychain;
mod models;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use api::{edit_image, generate_image, output_path_for_mime};
use error::AppError;
use keychain::{mask_api_key, read_api_key_from_keychain, save_api_key_to_keychain};
use models::list_image_models;

const DEFAULT_GEN_MODEL: &str = "gemini-2.0-flash-exp-image-generation";
const DEFAULT_EDIT_MODEL: &str = "nano-banana-pro-preview";
const DEFAULT_OUTPUT: &str = "./fingo-out.jpg";

#[derive(Parser, Debug)]
#[command(name = "fingo", about = "Gemini image generation and editing CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Gen {
        prompt: String,
        #[arg(short, long, default_value = DEFAULT_GEN_MODEL)]
        model: String,
        #[arg(short, long, default_value = DEFAULT_OUTPUT)]
        output: PathBuf,
    },
    Edit {
        image: PathBuf,
        prompt: String,
        #[arg(short, long, default_value = DEFAULT_EDIT_MODEL)]
        model: String,
        #[arg(short, long, default_value = DEFAULT_OUTPUT)]
        output: PathBuf,
    },
    Remove {
        image: PathBuf,
        prompt: String,
        #[arg(short, long, default_value = DEFAULT_EDIT_MODEL)]
        model: String,
        #[arg(short, long, default_value = DEFAULT_OUTPUT)]
        output: PathBuf,
    },
    Models,
    Key {
        #[command(subcommand)]
        command: KeyCommand,
    },
}

#[derive(Subcommand, Debug)]
enum KeyCommand {
    Save { api_key: String },
    Show,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            if !matches!(err, AppError::Silent) {
                eprintln!("{err}");
            }
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<(), AppError> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Gen {
            prompt,
            model,
            output,
        } => {
            let key = read_api_key_from_keychain()?;
            let generated = generate_image(&model, &key, &prompt)?;
            let output_path = output_path_for_mime(&output, &generated.mime_type);
            std::fs::write(&output_path, generated.bytes)
                .map_err(|err| AppError::Message(format!("Error: failed to save image: {err}")))?;
            println!("Saved: {}", output_path.display());
            Ok(())
        }
        Commands::Edit {
            image,
            prompt,
            model,
            output,
        }
        | Commands::Remove {
            image,
            prompt,
            model,
            output,
        } => {
            let key = read_api_key_from_keychain()?;
            let generated = edit_image(&model, &key, &prompt, &image)?;
            let output_path = output_path_for_mime(&output, &generated.mime_type);
            std::fs::write(&output_path, generated.bytes)
                .map_err(|err| AppError::Message(format!("Error: failed to save image: {err}")))?;
            println!("Saved: {}", output_path.display());
            Ok(())
        }
        Commands::Models => {
            let key = read_api_key_from_keychain()?;
            list_image_models(&key)
        }
        Commands::Key { command } => match command {
            KeyCommand::Save { api_key } => {
                save_api_key_to_keychain(&api_key)?;
                println!("API key saved to keychain");
                Ok(())
            }
            KeyCommand::Show => {
                let key = read_api_key_from_keychain()?;
                println!("{}", mask_api_key(&key));
                Ok(())
            }
        },
    }
}
