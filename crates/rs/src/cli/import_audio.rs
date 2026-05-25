use crate::error::{AppError, AppResult};
use crate::protocol::messages::{ImportAudioRequest, ImportAudioResponse, ImportAudioSound};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AudioManifest {
    to: Option<String>,
    sounds: Vec<AudioManifestSound>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AudioManifestSound {
    file: Option<PathBuf>,
    asset_id: String,
    name: Option<String>,
    volume: Option<f32>,
    playback_speed: Option<f32>,
    #[serde(default)]
    looped: bool,
}

#[allow(clippy::too_many_arguments)]
pub fn run(
    port: u16,
    studio: Option<String>,
    file: Option<PathBuf>,
    manifest: Option<PathBuf>,
    parent_path: Option<String>,
    name: Option<String>,
    asset_id: Option<String>,
    volume: Option<f32>,
    playback_speed: Option<f32>,
    looped: bool,
    json: bool,
) -> AppResult<()> {
    let (parent_path, sounds) = if let Some(manifest) = manifest {
        let manifest_dir = manifest
            .parent()
            .map(std::path::Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        let manifest: AudioManifest = serde_json::from_str(&std::fs::read_to_string(&manifest)?)?;
        let parent_path = parent_path
            .or(manifest.to)
            .unwrap_or_else(|| "SoundService".to_string());
        let sounds = manifest
            .sounds
            .into_iter()
            .map(|sound| {
                if let Some(file) = &sound.file {
                    let path = manifest_dir.join(file);
                    if !path.exists() {
                        return Err(AppError::Other(format!(
                            "audio manifest file does not exist: {}",
                            path.display()
                        )));
                    }
                }
                Ok(ImportAudioSound {
                    name: sound.name.unwrap_or_else(|| {
                        sound
                            .file
                            .as_ref()
                            .and_then(|path| path.file_stem())
                            .and_then(|value| value.to_str())
                            .unwrap_or("Sound")
                            .to_string()
                    }),
                    asset_id: sound.asset_id,
                    volume: sound.volume,
                    playback_speed: sound.playback_speed,
                    looped: sound.looped,
                })
            })
            .collect::<AppResult<Vec<_>>>()?;
        (parent_path, sounds)
    } else {
        let file =
            file.ok_or_else(|| AppError::Other("--file or --manifest is required".into()))?;
        if !file.exists() {
            return Err(AppError::Other(format!(
                "audio file does not exist: {}",
                file.display()
            )));
        }
        let asset_id = asset_id.ok_or_else(|| {
            AppError::Other("--asset-id is required for local audio files; rs does not fake local SoundId imports".into())
        })?;
        let name = name.unwrap_or_else(|| {
            file.file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("Sound")
                .to_string()
        });
        (
            parent_path.unwrap_or_else(|| "SoundService".to_string()),
            vec![ImportAudioSound {
                name,
                asset_id,
                volume,
                playback_speed,
                looped,
            }],
        )
    };

    let response: ImportAudioResponse = crate::cli::request::post(
        port,
        "import-audio",
        "/import-audio",
        &ImportAudioRequest {
            studio,
            parent_path,
            sounds,
            source_id: None,
        },
        75,
    )?;
    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        println!(
            "Imported {} Sound instance(s) under {}",
            response.sound_count, response.parent_path
        );
        for path in response.sound_paths {
            println!("  - {path}");
        }
        if !response.warnings.is_empty() {
            println!("Warnings ({}):", response.warnings.len());
            for warning in response.warnings {
                println!("  - {warning}");
            }
        }
    }
    Ok(())
}
