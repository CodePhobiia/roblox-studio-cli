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
    let (parent_path, sounds, source_id) = if let Some(manifest) = manifest {
        let manifest_dir = manifest
            .parent()
            .map(std::path::Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        let manifest_text = std::fs::read_to_string(&manifest)?;
        let source_id = crate::cli::import_uploaded::stable_source_id(
            "audio-manifest",
            &[
                manifest
                    .canonicalize()
                    .unwrap_or_else(|_| manifest.clone())
                    .to_string_lossy()
                    .replace('\\', "/"),
                format!("{:016x}", fnv1a64(manifest_text.as_bytes())),
            ],
        );
        let manifest: AudioManifest = serde_json::from_str(&manifest_text)?;
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
                    asset_id: crate::cli::import_uploaded::normalize_asset_id(&sound.asset_id)?,
                    volume: checked_volume(sound.volume)?,
                    playback_speed: checked_playback_speed(sound.playback_speed)?,
                    looped: sound.looped,
                })
            })
            .collect::<AppResult<Vec<_>>>()?;
        (parent_path, sounds, source_id)
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
        let asset_id = crate::cli::import_uploaded::normalize_asset_id(&asset_id)?;
        let volume = checked_volume(volume)?;
        let playback_speed = checked_playback_speed(playback_speed)?;
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
                asset_id: asset_id.clone(),
                volume,
                playback_speed,
                looped,
            }],
            crate::cli::import_uploaded::stable_source_id(
                "audio-file",
                &[
                    crate::cli::import_uploaded::stable_file_source_id("audio-file-source", &file)?,
                    asset_id,
                ],
            ),
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
            source_id: Some(source_id),
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

fn checked_volume(volume: Option<f32>) -> AppResult<Option<f32>> {
    crate::cli::import_uploaded::validate_sound_options(volume, None)?;
    Ok(volume)
}

fn checked_playback_speed(playback_speed: Option<f32>) -> AppResult<Option<f32>> {
    crate::cli::import_uploaded::validate_sound_options(None, playback_speed)?;
    Ok(playback_speed)
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::{checked_playback_speed, checked_volume};

    #[test]
    fn rejects_invalid_audio_ranges_before_studio() {
        assert!(checked_volume(Some(-0.1)).is_err());
        assert!(checked_volume(Some(10.1)).is_err());
        assert!(checked_volume(Some(f32::INFINITY)).is_err());
        assert!(checked_playback_speed(Some(0.0)).is_err());
        assert!(checked_playback_speed(Some(f32::NAN)).is_err());
    }
}
