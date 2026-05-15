use crate::bridge::auto_spawn::ensure_bridge_running;
use crate::error::{AppError, AppResult};
use crate::protocol::messages::{Envelope, ImportAssetRequest, ImportAssetResponse, ImportMesh};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub fn run(
    port: u16,
    studio: Option<String>,
    file: PathBuf,
    parent_path: String,
    name: Option<String>,
    scale: f32,
    anchored: bool,
    weld: bool,
) -> AppResult<()> {
    if !scale.is_finite() || scale <= 0.0 {
        return Err(AppError::Other(
            "--scale must be a positive finite number".into(),
        ));
    }

    let import_name = name.unwrap_or_else(|| file_stem(&file));
    let meshes = load_mesh_file(&file, scale)?;
    if meshes.is_empty() {
        return Err(AppError::Other(
            "asset contained no importable meshes".into(),
        ));
    }

    ensure_bridge_running(port)?;
    let url = format!("http://127.0.0.1:{port}/import-asset");
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(210))
        .build()?;
    let resp = client
        .post(&url)
        .json(&ImportAssetRequest {
            studio,
            parent_path,
            name: import_name,
            meshes,
            anchored,
            weld,
        })
        .send()
        .map_err(|source| AppError::BridgeUnreachable {
            url: url.clone(),
            source,
        })?;
    let env: Envelope<ImportAssetResponse> = resp.json()?;
    if !env.ok {
        return Err(crate::cli::envelope_error(
            "import-asset",
            env.error,
            env.code,
        ));
    }

    let response = env
        .data
        .ok_or_else(|| AppError::Other("import-asset returned no data".into()))?;
    println!(
        "Imported {} mesh(es), {} part(s), {} weld(s) at {}",
        response.mesh_count, response.part_count, response.weld_count, response.root_path
    );
    println!(
        "Geometry: {} vertices, {} triangles",
        response.vertex_count, response.triangle_count
    );
    if !response.warnings.is_empty() {
        println!("Warnings ({}):", response.warnings.len());
        for warning in response.warnings.iter().take(20) {
            println!("  - {warning}");
        }
        if response.warnings.len() > 20 {
            println!("  ... ({} more)", response.warnings.len() - 20);
        }
    }
    std::io::stdout().flush()?;
    Ok(())
}

fn file_stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("ImportedAsset")
        .to_string()
}

fn load_mesh_file(path: &Path, scale: f32) -> AppResult<Vec<ImportMesh>> {
    let ext = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "obj" => parse_obj(&fs::read_to_string(path)?, &file_stem(path), scale),
        "stl" => parse_stl(&fs::read(path)?, &file_stem(path), scale),
        "glb" => parse_glb(&fs::read(path)?, &file_stem(path), scale),
        "gltf" => parse_gltf(path, scale),
        _ => Err(AppError::Other(format!(
            "unsupported 3D asset format '.{ext}'. Supported formats: .obj, .stl, .gltf, .glb"
        ))),
    }
}

#[derive(Debug)]
struct RawObjMesh {
    name: String,
    triangles: Vec<[usize; 3]>,
}

fn parse_obj(source: &str, fallback_name: &str, scale: f32) -> AppResult<Vec<ImportMesh>> {
    let mut vertices = Vec::<[f32; 3]>::new();
    let mut meshes = Vec::<RawObjMesh>::new();
    let mut current = RawObjMesh {
        name: fallback_name.to_string(),
        triangles: Vec::new(),
    };

    for (line_index, line) in source.lines().enumerate() {
        let line_no = line_index + 1;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let Some(kind) = parts.next() else {
            continue;
        };
        match kind {
            "v" => {
                let x = parse_f32(parts.next(), line_no, "x")?;
                let y = parse_f32(parts.next(), line_no, "y")?;
                let z = parse_f32(parts.next(), line_no, "z")?;
                vertices.push([x, y, z]);
            }
            "o" | "g" => {
                let next_name = parts.collect::<Vec<_>>().join(" ");
                let next_name = if next_name.trim().is_empty() {
                    fallback_name.to_string()
                } else {
                    next_name
                };
                if current.triangles.is_empty() {
                    current.name = next_name;
                } else {
                    meshes.push(current);
                    current = RawObjMesh {
                        name: next_name,
                        triangles: Vec::new(),
                    };
                }
            }
            "f" => {
                let indices = parts
                    .map(|token| parse_obj_index(token, vertices.len(), line_no))
                    .collect::<AppResult<Vec<_>>>()?;
                if indices.len() < 3 {
                    return Err(AppError::Other(format!(
                        "OBJ face on line {line_no} has fewer than 3 vertices"
                    )));
                }
                for i in 1..indices.len() - 1 {
                    current
                        .triangles
                        .push([indices[0], indices[i], indices[i + 1]]);
                }
            }
            _ => {}
        }
    }

    if !current.triangles.is_empty() {
        meshes.push(current);
    }
    meshes
        .into_iter()
        .map(|mesh| build_mesh(mesh.name, &vertices, &mesh.triangles, scale))
        .collect()
}

fn parse_f32(value: Option<&str>, line_no: usize, component: &str) -> AppResult<f32> {
    let value = value.ok_or_else(|| {
        AppError::Other(format!("missing {component} component on line {line_no}"))
    })?;
    value.parse::<f32>().map_err(|_| {
        AppError::Other(format!(
            "invalid {component} component '{value}' on line {line_no}"
        ))
    })
}

fn parse_obj_index(token: &str, vertex_count: usize, line_no: usize) -> AppResult<usize> {
    let raw = token.split('/').next().unwrap_or("");
    let index = raw
        .parse::<isize>()
        .map_err(|_| AppError::Other(format!("invalid OBJ index '{raw}' on line {line_no}")))?;
    if index == 0 {
        return Err(AppError::Other(format!(
            "OBJ indices are 1-based; found 0 on line {line_no}"
        )));
    }

    let resolved = if index > 0 {
        index - 1
    } else {
        vertex_count as isize + index
    };
    if resolved < 0 || resolved >= vertex_count as isize {
        return Err(AppError::Other(format!(
            "OBJ index '{raw}' on line {line_no} is out of bounds"
        )));
    }
    Ok(resolved as usize)
}

fn build_mesh(
    name: String,
    source_vertices: &[[f32; 3]],
    source_triangles: &[[usize; 3]],
    scale: f32,
) -> AppResult<ImportMesh> {
    let mut remap = BTreeMap::<usize, usize>::new();
    let mut vertices = Vec::<[f32; 3]>::new();
    let mut triangles = Vec::<[usize; 3]>::new();

    for source_triangle in source_triangles {
        let mut triangle = [0usize; 3];
        for (slot, source_index) in source_triangle.iter().enumerate() {
            let next_index = vertices.len();
            let target_index = *remap.entry(*source_index).or_insert_with(|| {
                let [x, y, z] = source_vertices[*source_index];
                vertices.push([x * scale, y * scale, z * scale]);
                next_index
            });
            triangle[slot] = target_index + 1;
        }
        triangles.push(triangle);
    }

    Ok(ImportMesh {
        name,
        vertices,
        triangles,
    })
}

fn parse_stl(bytes: &[u8], fallback_name: &str, scale: f32) -> AppResult<Vec<ImportMesh>> {
    let mesh = if is_binary_stl(bytes) {
        parse_binary_stl(bytes, fallback_name, scale)?
    } else {
        parse_ascii_stl(bytes, fallback_name, scale)?
    };
    Ok(vec![mesh])
}

fn is_binary_stl(bytes: &[u8]) -> bool {
    if bytes.len() < 84 {
        return false;
    }
    let count = u32::from_le_bytes([bytes[80], bytes[81], bytes[82], bytes[83]]) as usize;
    84usize.saturating_add(count.saturating_mul(50)) == bytes.len()
}

fn parse_binary_stl(bytes: &[u8], fallback_name: &str, scale: f32) -> AppResult<ImportMesh> {
    let triangle_count = u32::from_le_bytes([bytes[80], bytes[81], bytes[82], bytes[83]]) as usize;
    let mut vertices = Vec::<[f32; 3]>::with_capacity(triangle_count * 3);
    let mut triangles = Vec::<[usize; 3]>::with_capacity(triangle_count);
    let mut cursor = 84usize;

    for triangle_index in 0..triangle_count {
        cursor += 12;
        let first = vertices.len() + 1;
        for _ in 0..3 {
            let x = read_f32_le(bytes, cursor)? * scale;
            let y = read_f32_le(bytes, cursor + 4)? * scale;
            let z = read_f32_le(bytes, cursor + 8)? * scale;
            vertices.push([x, y, z]);
            cursor += 12;
        }
        triangles.push([first, first + 1, first + 2]);
        cursor += 2;
        if cursor > bytes.len() && triangle_index + 1 < triangle_count {
            return Err(AppError::Other("truncated binary STL".into()));
        }
    }

    Ok(ImportMesh {
        name: fallback_name.to_string(),
        vertices,
        triangles,
    })
}

fn read_f32_le(bytes: &[u8], offset: usize) -> AppResult<f32> {
    let end = offset + 4;
    if end > bytes.len() {
        return Err(AppError::Other("truncated binary STL".into()));
    }
    Ok(f32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ]))
}

fn parse_ascii_stl(bytes: &[u8], fallback_name: &str, scale: f32) -> AppResult<ImportMesh> {
    let source = std::str::from_utf8(bytes)
        .map_err(|_| AppError::Other("STL is neither valid binary nor UTF-8 ASCII".into()))?;
    let mut vertices = Vec::<[f32; 3]>::new();
    let mut triangles = Vec::<[usize; 3]>::new();

    for (line_index, line) in source.lines().enumerate() {
        let line_no = line_index + 1;
        let mut parts = line.split_whitespace();
        if parts.next() != Some("vertex") {
            continue;
        }
        let x = parse_f32(parts.next(), line_no, "x")? * scale;
        let y = parse_f32(parts.next(), line_no, "y")? * scale;
        let z = parse_f32(parts.next(), line_no, "z")? * scale;
        vertices.push([x, y, z]);
        if vertices.len() % 3 == 0 {
            let first = vertices.len() - 2;
            triangles.push([first, first + 1, first + 2]);
        }
    }

    if triangles.is_empty() {
        return Err(AppError::Other("ASCII STL contained no triangles".into()));
    }

    Ok(ImportMesh {
        name: fallback_name.to_string(),
        vertices,
        triangles,
    })
}

#[derive(Debug, Clone)]
struct GltfBufferView {
    buffer: usize,
    byte_offset: usize,
    byte_length: usize,
    byte_stride: Option<usize>,
}

fn parse_glb(bytes: &[u8], fallback_name: &str, scale: f32) -> AppResult<Vec<ImportMesh>> {
    if bytes.len() < 20 || &bytes[0..4] != b"glTF" {
        return Err(AppError::Other("invalid GLB header".into()));
    }
    let version = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    if version != 2 {
        return Err(AppError::Other(format!(
            "unsupported GLB version {version}; expected 2"
        )));
    }
    let declared_len = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
    if declared_len != bytes.len() {
        return Err(AppError::Other(
            "GLB declared length does not match file size".into(),
        ));
    }

    let mut cursor = 12usize;
    let mut json = None;
    let mut bin = None;
    while cursor + 8 <= bytes.len() {
        let chunk_len = u32::from_le_bytes([
            bytes[cursor],
            bytes[cursor + 1],
            bytes[cursor + 2],
            bytes[cursor + 3],
        ]) as usize;
        let chunk_type = u32::from_le_bytes([
            bytes[cursor + 4],
            bytes[cursor + 5],
            bytes[cursor + 6],
            bytes[cursor + 7],
        ]);
        cursor += 8;
        let end = cursor + chunk_len;
        if end > bytes.len() {
            return Err(AppError::Other("truncated GLB chunk".into()));
        }
        match chunk_type {
            0x4E4F534A => json = Some(bytes[cursor..end].to_vec()),
            0x004E4942 => bin = Some(bytes[cursor..end].to_vec()),
            _ => {}
        }
        cursor = end;
    }

    let json = json.ok_or_else(|| AppError::Other("GLB missing JSON chunk".into()))?;
    let document: serde_json::Value = serde_json::from_slice(&json)?;
    let buffers = vec![bin.unwrap_or_default()];
    parse_gltf_document(&document, buffers, fallback_name, scale)
}

fn parse_gltf(path: &Path, scale: f32) -> AppResult<Vec<ImportMesh>> {
    let document: serde_json::Value = serde_json::from_str(&fs::read_to_string(path)?)?;
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let buffers = load_gltf_buffers(&document, parent)?;
    parse_gltf_document(&document, buffers, &file_stem(path), scale)
}

fn load_gltf_buffers(document: &serde_json::Value, parent: &Path) -> AppResult<Vec<Vec<u8>>> {
    let Some(buffers) = document.get("buffers").and_then(|value| value.as_array()) else {
        return Err(AppError::Other("glTF missing buffers".into()));
    };

    buffers
        .iter()
        .enumerate()
        .map(|(index, buffer)| {
            let uri = buffer
                .get("uri")
                .and_then(|value| value.as_str())
                .ok_or_else(|| AppError::Other(format!("glTF buffer {index} missing uri")))?;
            if let Some(encoded) = uri.strip_prefix("data:") {
                let comma = encoded.find(',').ok_or_else(|| {
                    AppError::Other(format!("glTF buffer {index} has invalid data URI"))
                })?;
                let (meta, data) = encoded.split_at(comma);
                if !meta.ends_with(";base64") {
                    return Err(AppError::Other(format!(
                        "glTF buffer {index} data URI is not base64"
                    )));
                }
                decode_base64(&data[1..])
            } else {
                fs::read(parent.join(uri.replace('\\', "/"))).map_err(AppError::Io)
            }
        })
        .collect()
}

fn parse_gltf_document(
    document: &serde_json::Value,
    buffers: Vec<Vec<u8>>,
    fallback_name: &str,
    scale: f32,
) -> AppResult<Vec<ImportMesh>> {
    let buffer_views = parse_gltf_buffer_views(document)?;
    let meshes = document
        .get("meshes")
        .and_then(|value| value.as_array())
        .ok_or_else(|| AppError::Other("glTF missing meshes".into()))?;
    let mut imported = Vec::<ImportMesh>::new();

    for (mesh_index, mesh) in meshes.iter().enumerate() {
        let mesh_name = mesh
            .get("name")
            .and_then(|value| value.as_str())
            .unwrap_or(fallback_name);
        let Some(primitives) = mesh.get("primitives").and_then(|value| value.as_array()) else {
            continue;
        };
        for (primitive_index, primitive) in primitives.iter().enumerate() {
            let mode = primitive
                .get("mode")
                .and_then(|value| value.as_u64())
                .unwrap_or(4);
            if mode != 4 {
                continue;
            }
            let position_accessor = primitive
                .get("attributes")
                .and_then(|value| value.get("POSITION"))
                .and_then(|value| value.as_u64())
                .ok_or_else(|| {
                    AppError::Other(format!(
                        "glTF mesh {mesh_index} primitive {primitive_index} missing POSITION"
                    ))
                })? as usize;
            let vertices =
                read_gltf_positions(document, &buffer_views, &buffers, position_accessor, scale)?;
            let triangles = if let Some(index_accessor) =
                primitive.get("indices").and_then(|value| value.as_u64())
            {
                read_gltf_indices(document, &buffer_views, &buffers, index_accessor as usize)?
            } else {
                sequential_triangles(vertices.len())?
            };
            if triangles.is_empty() {
                continue;
            }
            let name = if primitives.len() == 1 {
                mesh_name.to_string()
            } else {
                format!("{mesh_name}_{primitive_index}")
            };
            imported.push(ImportMesh {
                name,
                vertices,
                triangles,
            });
        }
    }

    if imported.is_empty() {
        return Err(AppError::Other(
            "glTF contained no triangle mesh primitives".into(),
        ));
    }
    Ok(imported)
}

fn parse_gltf_buffer_views(document: &serde_json::Value) -> AppResult<Vec<GltfBufferView>> {
    let Some(views) = document
        .get("bufferViews")
        .and_then(|value| value.as_array())
    else {
        return Err(AppError::Other("glTF missing bufferViews".into()));
    };
    views
        .iter()
        .enumerate()
        .map(|(index, view)| {
            Ok(GltfBufferView {
                buffer: json_usize(view, "buffer", &format!("bufferView {index}"))?,
                byte_offset: json_usize_default(view, "byteOffset", 0)?,
                byte_length: json_usize(view, "byteLength", &format!("bufferView {index}"))?,
                byte_stride: view
                    .get("byteStride")
                    .and_then(|value| value.as_u64())
                    .map(|value| value as usize),
            })
        })
        .collect()
}

fn read_gltf_positions(
    document: &serde_json::Value,
    buffer_views: &[GltfBufferView],
    buffers: &[Vec<u8>],
    accessor_index: usize,
    scale: f32,
) -> AppResult<Vec<[f32; 3]>> {
    let accessor = gltf_accessor(document, accessor_index)?;
    let component_type = json_usize(accessor, "componentType", "POSITION accessor")?;
    let value_type = accessor
        .get("type")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    if component_type != 5126 || value_type != "VEC3" {
        return Err(AppError::Other(
            "glTF POSITION accessor must be FLOAT VEC3".into(),
        ));
    }
    let count = json_usize(accessor, "count", "POSITION accessor")?;
    let view_index = json_usize(accessor, "bufferView", "POSITION accessor")?;
    let view = buffer_views
        .get(view_index)
        .ok_or_else(|| AppError::Other("POSITION accessor references missing bufferView".into()))?;
    let buffer = buffers
        .get(view.buffer)
        .ok_or_else(|| AppError::Other("POSITION bufferView references missing buffer".into()))?;
    let accessor_offset = json_usize_default(accessor, "byteOffset", 0)?;
    let stride = view.byte_stride.unwrap_or(12);
    let base = view.byte_offset + accessor_offset;
    validate_gltf_range(
        view,
        accessor_offset,
        count,
        stride,
        12,
        "POSITION accessor",
    )?;
    let mut vertices = Vec::with_capacity(count);
    for index in 0..count {
        let offset = base + index * stride;
        let x = read_f32_le(buffer, offset)? * scale;
        let y = read_f32_le(buffer, offset + 4)? * scale;
        let z = read_f32_le(buffer, offset + 8)? * scale;
        vertices.push([x, y, z]);
    }
    Ok(vertices)
}

fn read_gltf_indices(
    document: &serde_json::Value,
    buffer_views: &[GltfBufferView],
    buffers: &[Vec<u8>],
    accessor_index: usize,
) -> AppResult<Vec<[usize; 3]>> {
    let accessor = gltf_accessor(document, accessor_index)?;
    let component_type = json_usize(accessor, "componentType", "index accessor")?;
    let count = json_usize(accessor, "count", "index accessor")?;
    if count % 3 != 0 {
        return Err(AppError::Other(
            "glTF index count is not divisible by 3".into(),
        ));
    }
    let view_index = json_usize(accessor, "bufferView", "index accessor")?;
    let view = buffer_views
        .get(view_index)
        .ok_or_else(|| AppError::Other("index accessor references missing bufferView".into()))?;
    let buffer = buffers
        .get(view.buffer)
        .ok_or_else(|| AppError::Other("index bufferView references missing buffer".into()))?;
    let accessor_offset = json_usize_default(accessor, "byteOffset", 0)?;
    let component_size = match component_type {
        5121 => 1,
        5123 => 2,
        5125 => 4,
        _ => {
            return Err(AppError::Other(format!(
                "unsupported glTF index componentType {component_type}"
            )))
        }
    };
    let stride = view.byte_stride.unwrap_or(component_size);
    let base = view.byte_offset + accessor_offset;
    validate_gltf_range(
        view,
        accessor_offset,
        count,
        stride,
        component_size,
        "index accessor",
    )?;
    let mut values = Vec::with_capacity(count);
    for index in 0..count {
        let offset = base + index * stride;
        let value = match component_type {
            5121 => *buffer
                .get(offset)
                .ok_or_else(|| AppError::Other("truncated glTF index buffer".into()))?
                as usize,
            5123 => read_u16_le(buffer, offset)? as usize,
            5125 => read_u32_le(buffer, offset)? as usize,
            _ => unreachable!(),
        };
        values.push(value + 1);
    }
    Ok(values
        .chunks_exact(3)
        .map(|chunk| [chunk[0], chunk[1], chunk[2]])
        .collect())
}

fn sequential_triangles(vertex_count: usize) -> AppResult<Vec<[usize; 3]>> {
    if vertex_count % 3 != 0 {
        return Err(AppError::Other(
            "non-indexed glTF primitive vertex count is not divisible by 3".into(),
        ));
    }
    Ok((0..vertex_count / 3)
        .map(|triangle| {
            let first = triangle * 3 + 1;
            [first, first + 1, first + 2]
        })
        .collect())
}

fn gltf_accessor(document: &serde_json::Value, index: usize) -> AppResult<&serde_json::Value> {
    document
        .get("accessors")
        .and_then(|value| value.as_array())
        .and_then(|accessors| accessors.get(index))
        .ok_or_else(|| AppError::Other(format!("glTF accessor {index} not found")))
}

fn json_usize(value: &serde_json::Value, key: &str, context: &str) -> AppResult<usize> {
    value
        .get(key)
        .and_then(|value| value.as_u64())
        .map(|value| value as usize)
        .ok_or_else(|| AppError::Other(format!("{context} missing {key}")))
}

fn json_usize_default(value: &serde_json::Value, key: &str, default: usize) -> AppResult<usize> {
    Ok(value
        .get(key)
        .and_then(|value| value.as_u64())
        .map(|value| value as usize)
        .unwrap_or(default))
}

fn validate_gltf_range(
    view: &GltfBufferView,
    accessor_offset: usize,
    count: usize,
    stride: usize,
    element_size: usize,
    context: &str,
) -> AppResult<()> {
    if count == 0 {
        return Ok(());
    }
    let relative_end = accessor_offset
        .saturating_add((count - 1).saturating_mul(stride))
        .saturating_add(element_size);
    if relative_end > view.byte_length {
        return Err(AppError::Other(format!(
            "{context} exceeds its bufferView byteLength"
        )));
    }
    Ok(())
}

fn read_u16_le(bytes: &[u8], offset: usize) -> AppResult<u16> {
    let end = offset + 2;
    if end > bytes.len() {
        return Err(AppError::Other("truncated glTF buffer".into()));
    }
    Ok(u16::from_le_bytes([bytes[offset], bytes[offset + 1]]))
}

fn read_u32_le(bytes: &[u8], offset: usize) -> AppResult<u32> {
    let end = offset + 4;
    if end > bytes.len() {
        return Err(AppError::Other("truncated glTF buffer".into()));
    }
    Ok(u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ]))
}

fn decode_base64(input: &str) -> AppResult<Vec<u8>> {
    let mut out = Vec::new();
    let mut buffer = 0u32;
    let mut bits = 0u8;

    for byte in input.bytes() {
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => break,
            b'\r' | b'\n' | b'\t' | b' ' => continue,
            _ => {
                return Err(AppError::Other(
                    "invalid base64 character in data URI".into(),
                ))
            }
        } as u32;
        buffer = (buffer << 6) | value;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(((buffer >> bits) & 0xff) as u8);
        }
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::{parse_gltf_document, parse_obj, parse_stl};

    #[test]
    fn obj_quad_triangulates() {
        let source = "\
v 0 0 0
v 1 0 0
v 1 1 0
v 0 1 0
f 1 2 3 4
";
        let meshes = parse_obj(source, "quad", 2.0).unwrap();
        assert_eq!(meshes.len(), 1);
        assert_eq!(meshes[0].vertices.len(), 4);
        assert_eq!(meshes[0].vertices[1], [2.0, 0.0, 0.0]);
        assert_eq!(meshes[0].triangles, vec![[1, 2, 3], [1, 3, 4]]);
    }

    #[test]
    fn obj_groups_become_meshes() {
        let source = "\
o A
v 0 0 0
v 1 0 0
v 0 1 0
f 1 2 3
o B
v 0 0 1
f 1 3 4
";
        let meshes = parse_obj(source, "asset", 1.0).unwrap();
        assert_eq!(meshes.len(), 2);
        assert_eq!(meshes[0].name, "A");
        assert_eq!(meshes[1].name, "B");
    }

    #[test]
    fn ascii_stl_imports_triangle() {
        let source = b"solid tri
facet normal 0 0 1
outer loop
vertex 0 0 0
vertex 1 0 0
vertex 0 1 0
endloop
endfacet
endsolid tri
";
        let meshes = parse_stl(source, "tri", 1.0).unwrap();
        assert_eq!(meshes.len(), 1);
        assert_eq!(meshes[0].triangles, vec![[1, 2, 3]]);
    }

    #[test]
    fn gltf_document_imports_indexed_triangle() {
        let mut buffer = Vec::new();
        for value in [0.0f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0] {
            buffer.extend_from_slice(&value.to_le_bytes());
        }
        for value in [0u16, 1, 2] {
            buffer.extend_from_slice(&value.to_le_bytes());
        }
        let document = serde_json::json!({
            "buffers": [{ "byteLength": buffer.len() }],
            "bufferViews": [
                { "buffer": 0, "byteOffset": 0, "byteLength": 36 },
                { "buffer": 0, "byteOffset": 36, "byteLength": 6 }
            ],
            "accessors": [
                { "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" },
                { "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }
            ],
            "meshes": [{
                "name": "Tri",
                "primitives": [{
                    "attributes": { "POSITION": 0 },
                    "indices": 1
                }]
            }]
        });
        let meshes = parse_gltf_document(&document, vec![buffer], "asset", 1.0).unwrap();
        assert_eq!(meshes.len(), 1);
        assert_eq!(meshes[0].name, "Tri");
        assert_eq!(meshes[0].vertices.len(), 3);
        assert_eq!(meshes[0].triangles, vec![[1, 2, 3]]);
    }
}
