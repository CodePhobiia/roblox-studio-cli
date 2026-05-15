use crate::bridge::auto_spawn::ensure_bridge_running;
use crate::error::{AppError, AppResult};
use crate::protocol::messages::{Envelope, ImportImageRequest, ImportImageResponse};
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

const MAX_EDITABLE_IMAGE_DIMENSION: u32 = 1024;

pub fn run(
    port: u16,
    studio: Option<String>,
    file: PathBuf,
    parent_path: String,
    name: Option<String>,
    kind: String,
    size: Option<String>,
    position: String,
) -> AppResult<()> {
    let import_name = name.unwrap_or_else(|| file_stem(&file));
    let image = load_png(&file)?;
    let (ui_width, ui_height) = match size {
        Some(value) => parse_size(&value)?,
        None if kind == "icon" => (64, 64),
        None => (image.width, image.height),
    };
    let (position_x, position_y) = parse_position(&position)?;

    ensure_bridge_running(port)?;
    let url = format!("http://127.0.0.1:{port}/import-image");
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(150))
        .build()?;
    let resp = client
        .post(&url)
        .json(&ImportImageRequest {
            studio,
            parent_path,
            name: import_name,
            kind,
            width: image.width,
            height: image.height,
            ui_width,
            ui_height,
            position_x,
            position_y,
            pixels_base64: encode_base64(&image.rgba),
        })
        .send()
        .map_err(|source| AppError::BridgeUnreachable {
            url: url.clone(),
            source,
        })?;
    let env: Envelope<ImportImageResponse> = resp.json()?;
    if !env.ok {
        return Err(crate::cli::envelope_error(
            "import-image",
            env.error,
            env.code,
        ));
    }

    let response = env
        .data
        .ok_or_else(|| AppError::Other("import-image returned no data".into()))?;
    println!(
        "Imported {}x{} PNG as {} at {}",
        response.width, response.height, response.class_name, response.image_path
    );
    println!("GUI root: {}", response.gui_path);
    let warnings = image
        .warnings
        .iter()
        .chain(response.warnings.iter())
        .collect::<Vec<_>>();
    if !warnings.is_empty() {
        println!("Warnings ({}):", warnings.len());
        for warning in warnings.iter().take(20) {
            println!("  - {warning}");
        }
        if warnings.len() > 20 {
            println!("  ... ({} more)", warnings.len() - 20);
        }
    }
    std::io::stdout().flush()?;
    Ok(())
}

struct PngImage {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
    warnings: Vec<String>,
}

fn file_stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("ImportedImage")
        .to_string()
}

fn load_png(path: &Path) -> AppResult<PngImage> {
    let file = File::open(path)?;
    let mut decoder = png::Decoder::new(BufReader::new(file));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder
        .read_info()
        .map_err(|err| AppError::Other(format!("could not read PNG header: {err}")))?;
    let mut buffer = vec![0; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut buffer)
        .map_err(|err| AppError::Other(format!("could not decode PNG: {err}")))?;
    let bytes = &buffer[..info.buffer_size()];
    let rgba = to_rgba(bytes, info.color_type)?;
    let mut image = PngImage {
        width: info.width,
        height: info.height,
        rgba,
        warnings: Vec::new(),
    };
    constrain_image_size(&mut image)?;
    Ok(image)
}

fn to_rgba(bytes: &[u8], color_type: png::ColorType) -> AppResult<Vec<u8>> {
    match color_type {
        png::ColorType::Rgba => Ok(bytes.to_vec()),
        png::ColorType::Rgb => {
            let mut rgba = Vec::with_capacity(bytes.len() / 3 * 4);
            for rgb in bytes.chunks_exact(3) {
                rgba.extend_from_slice(rgb);
                rgba.push(255);
            }
            Ok(rgba)
        }
        png::ColorType::Grayscale => Ok(bytes
            .iter()
            .flat_map(|gray| [*gray, *gray, *gray, 255])
            .collect()),
        png::ColorType::GrayscaleAlpha => {
            let mut rgba = Vec::with_capacity(bytes.len() / 2 * 4);
            for ga in bytes.chunks_exact(2) {
                rgba.extend_from_slice(&[ga[0], ga[0], ga[0], ga[1]]);
            }
            Ok(rgba)
        }
        png::ColorType::Indexed => Err(AppError::Other(
            "indexed PNG did not expand to RGB/RGBA during decode".into(),
        )),
    }
}

fn constrain_image_size(image: &mut PngImage) -> AppResult<()> {
    if image.width <= MAX_EDITABLE_IMAGE_DIMENSION && image.height <= MAX_EDITABLE_IMAGE_DIMENSION {
        return Ok(());
    }
    let scale = (MAX_EDITABLE_IMAGE_DIMENSION as f32 / image.width as f32)
        .min(MAX_EDITABLE_IMAGE_DIMENSION as f32 / image.height as f32);
    let new_width = ((image.width as f32 * scale).floor() as u32).max(1);
    let new_height = ((image.height as f32 * scale).floor() as u32).max(1);
    let resized = resize_nearest(
        &image.rgba,
        image.width,
        image.height,
        new_width,
        new_height,
    )?;
    image.warnings.push(format!(
        "resized PNG from {}x{} to {}x{} to fit EditableImage's {}px limit",
        image.width, image.height, new_width, new_height, MAX_EDITABLE_IMAGE_DIMENSION
    ));
    image.width = new_width;
    image.height = new_height;
    image.rgba = resized;
    Ok(())
}

fn resize_nearest(
    rgba: &[u8],
    width: u32,
    height: u32,
    new_width: u32,
    new_height: u32,
) -> AppResult<Vec<u8>> {
    if rgba.len() != width as usize * height as usize * 4 {
        return Err(AppError::Other(
            "RGBA buffer size does not match dimensions".into(),
        ));
    }
    let mut out = vec![0u8; new_width as usize * new_height as usize * 4];
    for y in 0..new_height {
        let source_y = (y as u64 * height as u64 / new_height as u64) as u32;
        for x in 0..new_width {
            let source_x = (x as u64 * width as u64 / new_width as u64) as u32;
            let source = ((source_y * width + source_x) * 4) as usize;
            let target = ((y * new_width + x) * 4) as usize;
            out[target..target + 4].copy_from_slice(&rgba[source..source + 4]);
        }
    }
    Ok(out)
}

fn parse_size(value: &str) -> AppResult<(u32, u32)> {
    let (w, h) = value
        .split_once('x')
        .or_else(|| value.split_once('X'))
        .ok_or_else(|| AppError::Other("--size must look like 64x64".into()))?;
    let width = parse_positive_u32(w, "--size width")?;
    let height = parse_positive_u32(h, "--size height")?;
    Ok((width, height))
}

fn parse_position(value: &str) -> AppResult<(i32, i32)> {
    let (x, y) = value
        .split_once(',')
        .ok_or_else(|| AppError::Other("--position must look like 0,0".into()))?;
    let x = x
        .trim()
        .parse::<i32>()
        .map_err(|_| AppError::Other("--position x must be an integer".into()))?;
    let y = y
        .trim()
        .parse::<i32>()
        .map_err(|_| AppError::Other("--position y must be an integer".into()))?;
    Ok((x, y))
}

fn parse_positive_u32(value: &str, label: &str) -> AppResult<u32> {
    let parsed = value
        .trim()
        .parse::<u32>()
        .map_err(|_| AppError::Other(format!("{label} must be a positive integer")))?;
    if parsed == 0 {
        return Err(AppError::Other(format!(
            "{label} must be greater than zero"
        )));
    }
    Ok(parsed)
}

fn encode_base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let a = chunk[0];
        let b = *chunk.get(1).unwrap_or(&0);
        let c = *chunk.get(2).unwrap_or(&0);
        out.push(TABLE[(a >> 2) as usize] as char);
        out.push(TABLE[(((a & 0b0000_0011) << 4) | (b >> 4)) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[(((b & 0b0000_1111) << 2) | (c >> 6)) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[(c & 0b0011_1111) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{encode_base64, load_png, parse_position, parse_size};
    use std::fs;

    #[test]
    fn parses_size_and_position() {
        assert_eq!(parse_size("32x64").unwrap(), (32, 64));
        assert_eq!(parse_position("-2,10").unwrap(), (-2, 10));
    }

    #[test]
    fn base64_encodes_padding() {
        assert_eq!(encode_base64(b""), "");
        assert_eq!(encode_base64(b"f"), "Zg==");
        assert_eq!(encode_base64(b"fo"), "Zm8=");
        assert_eq!(encode_base64(b"foo"), "Zm9v");
    }

    #[test]
    fn decodes_png_to_rgba() {
        let path = std::env::temp_dir().join(format!(
            "rs-import-image-test-{}-{}.png",
            std::process::id(),
            "rgba"
        ));
        let file = fs::File::create(&path).unwrap();
        let mut encoder = png::Encoder::new(file, 2, 1);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer
            .write_image_data(&[255, 0, 0, 255, 0, 255, 0, 128])
            .unwrap();
        writer.finish().unwrap();

        let image = load_png(&path).unwrap();
        assert_eq!(image.width, 2);
        assert_eq!(image.height, 1);
        assert_eq!(image.rgba, vec![255, 0, 0, 255, 0, 255, 0, 128]);
        let _ = fs::remove_file(path);
    }
}
