use crate::error::{AppError, AppResult};
use crate::protocol::messages::{ImportAssetRequest, ImportAssetResponse, ImportMesh};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn run(
    port: u16,
    studio: Option<String>,
    file: PathBuf,
    parent_path: String,
    name: Option<String>,
    scale: f32,
    anchored: bool,
    weld: bool,
    texture_root: Option<PathBuf>,
) -> AppResult<()> {
    if !scale.is_finite() || scale <= 0.0 {
        return Err(AppError::Other(
            "--scale must be a positive finite number".into(),
        ));
    }

    let import_name = name.unwrap_or_else(|| file_stem(&file));
    let source_id = crate::cli::import_uploaded::stable_file_source_id("asset-file", &file)?;
    let texture_resolver = TextureResolver::load(texture_root.as_deref())?;
    let meshes = load_mesh_file(&file, scale, &texture_resolver)?;
    if meshes.is_empty() {
        return Err(AppError::Other(
            "asset contained no importable meshes".into(),
        ));
    }

    let response: ImportAssetResponse = crate::cli::request::post(
        port,
        "import-asset",
        "/import-asset",
        &ImportAssetRequest {
            studio,
            parent_path,
            name: import_name,
            meshes,
            anchored,
            weld,
            source_id: Some(source_id),
        },
        210,
    )?;
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

fn load_mesh_file(
    path: &Path,
    scale: f32,
    texture_resolver: &TextureResolver,
) -> AppResult<Vec<ImportMesh>> {
    let ext = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "obj" => parse_obj_file(path, scale, texture_resolver),
        "stl" => parse_stl(&fs::read(path)?, &file_stem(path), scale),
        "glb" => parse_glb(&fs::read(path)?, &file_stem(path), scale, texture_resolver),
        "gltf" => parse_gltf(path, scale, texture_resolver),
        _ => parse_with_blender(path, scale).map_err(|err| {
            AppError::Other(format!(
                "unsupported native format '.{ext}', and Blender conversion failed: {err}"
            ))
        }),
    }
}

fn parse_with_blender(path: &Path, scale: f32) -> AppResult<Vec<ImportMesh>> {
    if !path.exists() {
        return Err(AppError::Other(format!(
            "source file does not exist: {}",
            path.display()
        )));
    }

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let work_dir =
        std::env::temp_dir().join(format!("rs-import-asset-{}-{stamp}", std::process::id()));
    fs::create_dir_all(&work_dir)?;
    let script_path = work_dir.join("convert.py");
    let out_path = work_dir.join("converted.glb");
    fs::write(&script_path, blender_script(path, &out_path).as_bytes())?;

    let output = run_blender(&script_path)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(AppError::Other(format!(
            "Blender exited with {}.\nstdout:\n{}\nstderr:\n{}",
            output.status,
            tail(&stdout, 40),
            tail(&stderr, 40)
        )));
    }
    if !out_path.exists() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(AppError::Other(format!(
            "Blender did not produce a converted GLB.\nstdout:\n{}\nstderr:\n{}",
            tail(&stdout, 40),
            tail(&stderr, 40)
        )));
    }

    let meshes = parse_glb(
        &fs::read(&out_path)?,
        &file_stem(path),
        scale,
        &TextureResolver::default(),
    )?;
    let _ = fs::remove_file(&script_path);
    let _ = fs::remove_file(&out_path);
    let _ = fs::remove_dir(&work_dir);
    Ok(meshes)
}

fn run_blender(script_path: &Path) -> AppResult<std::process::Output> {
    let mut candidates = Vec::<String>::new();
    if let Ok(path) = std::env::var("BLENDER") {
        if !path.trim().is_empty() {
            candidates.push(path);
        }
    }
    candidates.extend(discover_blender_with_where());
    candidates.extend([
        "blender".to_string(),
        "blender.exe".to_string(),
        "blender.cmd".to_string(),
    ]);

    let mut last_err = None;
    for candidate in candidates {
        let mut command = if is_shell_script(&candidate) {
            let mut command = Command::new("cmd");
            command.arg("/C").arg(&candidate);
            command
        } else {
            Command::new(&candidate)
        };
        match command
            .arg("--background")
            .arg("--factory-startup")
            .arg("--python")
            .arg(script_path)
            .output()
        {
            Ok(output) => return Ok(output),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                last_err = Some(format!("{candidate}: {err}"));
            }
            Err(err) => return Err(AppError::Other(format!("could not run {candidate}: {err}"))),
        }
    }

    Err(AppError::Other(format!(
        "could not run Blender. Set BLENDER to the Blender executable or convert this file to OBJ/STL/glTF/GLB first. Last error: {}",
        last_err.unwrap_or_else(|| "no candidates tried".into())
    )))
}

fn discover_blender_with_where() -> Vec<String> {
    let Ok(output) = Command::new("where.exe").arg("blender").output() else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn is_shell_script(candidate: &str) -> bool {
    let lower = candidate.to_ascii_lowercase();
    lower.ends_with(".cmd") || lower.ends_with(".bat")
}

fn blender_script(source: &Path, target: &Path) -> String {
    format!(
        r#"
import bpy
import os

source = {source}
target = {target}
ext = os.path.splitext(source)[1].lower()

def reset_scene():
    bpy.ops.object.select_all(action='SELECT')
    bpy.ops.object.delete()

def import_source():
    if ext == '.fbx':
        bpy.ops.import_scene.fbx(filepath=source)
    elif ext == '.dae':
        bpy.ops.wm.collada_import(filepath=source)
    elif ext == '.blend':
        bpy.ops.wm.open_mainfile(filepath=source)
    elif ext == '.ply':
        bpy.ops.wm.ply_import(filepath=source)
    elif ext == '.abc':
        bpy.ops.wm.alembic_import(filepath=source)
    elif ext in ('.usd', '.usda', '.usdc'):
        bpy.ops.wm.usd_import(filepath=source)
    elif ext == '.obj':
        bpy.ops.wm.obj_import(filepath=source)
    elif ext == '.stl':
        bpy.ops.wm.stl_import(filepath=source)
    elif ext in ('.gltf', '.glb'):
        bpy.ops.import_scene.gltf(filepath=source)
    else:
        raise RuntimeError('Blender import fallback does not support ' + ext)

reset_scene()
import_source()
mesh_objects = [obj for obj in bpy.context.scene.objects if obj.type == 'MESH']
if not mesh_objects:
    raise RuntimeError('No mesh objects were imported')

bpy.ops.object.select_all(action='DESELECT')
for obj in mesh_objects:
    obj.select_set(True)
bpy.context.view_layer.objects.active = mesh_objects[0]
bpy.ops.export_scene.gltf(filepath=target, export_format='GLB', use_selection=True)
"#,
        source = py_string(source),
        target = py_string(target)
    )
}

fn py_string(path: &Path) -> String {
    let value = path
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('\'', "\\'");
    format!("'{value}'")
}

fn tail(value: &str, max_lines: usize) -> String {
    let lines = value.lines().collect::<Vec<_>>();
    let start = lines.len().saturating_sub(max_lines);
    lines[start..].join("\n")
}

#[derive(Debug)]
struct RawObjMesh {
    name: String,
    triangles: Vec<[usize; 3]>,
    material_name: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct ObjMaterial {
    color: Option<[f32; 3]>,
    texture_uri: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct TextureResolver {
    root: Option<PathBuf>,
    map: BTreeMap<String, String>,
}

impl TextureResolver {
    fn load(root: Option<&Path>) -> AppResult<Self> {
        let Some(root) = root else {
            return Ok(Self::default());
        };
        let manifest = root.join("rs-textures.json");
        let map = if manifest.exists() {
            serde_json::from_str(&fs::read_to_string(manifest)?)?
        } else {
            BTreeMap::new()
        };
        Ok(Self {
            root: Some(root.to_path_buf()),
            map,
        })
    }

    fn resolve(&self, texture: &str, base: &Path) -> String {
        if let Some(mapped) = self.map.get(texture).or_else(|| {
            Path::new(texture)
                .file_name()
                .and_then(|v| v.to_str())
                .and_then(|name| self.map.get(name))
        }) {
            return mapped.clone();
        }
        if let Some(root) = &self.root {
            let name = Path::new(texture)
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or(texture);
            let candidate = root.join(name);
            if candidate.exists() {
                return candidate.to_string_lossy().replace('\\', "/");
            }
        }
        base.join(texture).to_string_lossy().replace('\\', "/")
    }
}

fn parse_obj_file(
    path: &Path,
    scale: f32,
    texture_resolver: &TextureResolver,
) -> AppResult<Vec<ImportMesh>> {
    let source = fs::read_to_string(path)?;
    let materials = load_obj_materials(path, &source, texture_resolver)?;
    parse_obj_with_materials(&source, &file_stem(path), scale, &materials)
}

#[cfg(test)]
fn parse_obj(source: &str, fallback_name: &str, scale: f32) -> AppResult<Vec<ImportMesh>> {
    parse_obj_with_materials(source, fallback_name, scale, &BTreeMap::new())
}

fn parse_obj_with_materials(
    source: &str,
    fallback_name: &str,
    scale: f32,
    materials: &BTreeMap<String, ObjMaterial>,
) -> AppResult<Vec<ImportMesh>> {
    let mut vertices = Vec::<[f32; 3]>::new();
    let mut meshes = Vec::<RawObjMesh>::new();
    let mut current_material: Option<String> = None;
    let mut current = RawObjMesh {
        name: fallback_name.to_string(),
        triangles: Vec::new(),
        material_name: None,
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
                        material_name: current_material.clone(),
                    };
                }
            }
            "usemtl" => {
                let next_material = parts.collect::<Vec<_>>().join(" ");
                let next_material = (!next_material.trim().is_empty()).then_some(next_material);
                if !current.triangles.is_empty() && current.material_name != next_material {
                    meshes.push(current);
                    let suffix = next_material.as_deref().unwrap_or("material");
                    current = RawObjMesh {
                        name: format!("{fallback_name}_{suffix}"),
                        triangles: Vec::new(),
                        material_name: next_material.clone(),
                    };
                } else {
                    current.material_name = next_material.clone();
                }
                current_material = next_material;
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
        .map(|mesh| {
            let material = mesh
                .material_name
                .as_ref()
                .and_then(|name| materials.get(name));
            build_mesh(
                mesh.name.clone(),
                &vertices,
                &mesh.triangles,
                scale,
                mesh.material_name.clone(),
                material.and_then(|value| value.texture_uri.clone()),
                material.and_then(|value| value.color),
                Some(mesh.name),
                None,
            )
        })
        .collect()
}

fn load_obj_materials(
    path: &Path,
    source: &str,
    texture_resolver: &TextureResolver,
) -> AppResult<BTreeMap<String, ObjMaterial>> {
    let mut materials = BTreeMap::new();
    for line in source.lines() {
        let line = line.trim();
        if !line.starts_with("mtllib ") {
            continue;
        }
        let name = line.trim_start_matches("mtllib").trim();
        if name.is_empty() {
            continue;
        }
        let mtl_path = path.parent().unwrap_or_else(|| Path::new(".")).join(name);
        if mtl_path.exists() {
            materials.extend(parse_mtl(
                &fs::read_to_string(&mtl_path)?,
                mtl_path.parent().unwrap_or_else(|| Path::new(".")),
                texture_resolver,
            )?);
        }
    }
    Ok(materials)
}

fn parse_mtl(
    source: &str,
    base: &Path,
    texture_resolver: &TextureResolver,
) -> AppResult<BTreeMap<String, ObjMaterial>> {
    let mut materials = BTreeMap::<String, ObjMaterial>::new();
    let mut current: Option<String> = None;
    for line in source.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        match parts.next() {
            Some("newmtl") => {
                let name = parts.collect::<Vec<_>>().join(" ");
                if !name.is_empty() {
                    materials.entry(name.clone()).or_default();
                    current = Some(name);
                }
            }
            Some("Kd") => {
                if let Some(name) = &current {
                    let color = [
                        parse_f32(parts.next(), 0, "Kd.r")?,
                        parse_f32(parts.next(), 0, "Kd.g")?,
                        parse_f32(parts.next(), 0, "Kd.b")?,
                    ];
                    materials.entry(name.clone()).or_default().color = Some(color);
                }
            }
            Some("map_Kd") => {
                if let Some(name) = &current {
                    let texture = parts.collect::<Vec<_>>().join(" ");
                    if !texture.is_empty() {
                        materials.entry(name.clone()).or_default().texture_uri =
                            Some(texture_resolver.resolve(&texture, base));
                    }
                }
            }
            _ => {}
        }
    }
    Ok(materials)
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
    material_name: Option<String>,
    texture_uri: Option<String>,
    color: Option<[f32; 3]>,
    hierarchy_path: Option<String>,
    source_pivot: Option<[f32; 3]>,
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
        material_name,
        texture_uri,
        color,
        hierarchy_path,
        source_pivot,
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
        material_name: None,
        texture_uri: None,
        color: None,
        hierarchy_path: Some(fallback_name.to_string()),
        source_pivot: None,
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
        material_name: None,
        texture_uri: None,
        color: None,
        hierarchy_path: Some(fallback_name.to_string()),
        source_pivot: None,
    })
}

#[derive(Debug, Clone, Copy)]
struct Mat4([f32; 16]);

impl Mat4 {
    fn identity() -> Self {
        Self([
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ])
    }

    fn mul(self, rhs: Self) -> Self {
        let mut out = [0.0f32; 16];
        for col in 0..4 {
            for row in 0..4 {
                out[col * 4 + row] = (0..4)
                    .map(|k| self.0[k * 4 + row] * rhs.0[col * 4 + k])
                    .sum();
            }
        }
        Self(out)
    }

    fn transform_point(self, point: [f32; 3]) -> [f32; 3] {
        let [x, y, z] = point;
        [
            self.0[0] * x + self.0[4] * y + self.0[8] * z + self.0[12],
            self.0[1] * x + self.0[5] * y + self.0[9] * z + self.0[13],
            self.0[2] * x + self.0[6] * y + self.0[10] * z + self.0[14],
        ]
    }

    fn translation(self) -> [f32; 3] {
        [self.0[12], self.0[13], self.0[14]]
    }
}

#[derive(Debug, Clone)]
struct GltfBufferView {
    buffer: usize,
    byte_offset: usize,
    byte_length: usize,
    byte_stride: Option<usize>,
}

fn parse_glb(
    bytes: &[u8],
    fallback_name: &str,
    scale: f32,
    texture_resolver: &TextureResolver,
) -> AppResult<Vec<ImportMesh>> {
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
    parse_gltf_document(&document, buffers, fallback_name, scale, texture_resolver)
}

fn parse_gltf(
    path: &Path,
    scale: f32,
    texture_resolver: &TextureResolver,
) -> AppResult<Vec<ImportMesh>> {
    let document: serde_json::Value = serde_json::from_str(&fs::read_to_string(path)?)?;
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let buffers = load_gltf_buffers(&document, parent)?;
    parse_gltf_document(
        &document,
        buffers,
        &file_stem(path),
        scale,
        texture_resolver,
    )
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
    texture_resolver: &TextureResolver,
) -> AppResult<Vec<ImportMesh>> {
    let buffer_views = parse_gltf_buffer_views(document)?;
    let meshes = document
        .get("meshes")
        .and_then(|value| value.as_array())
        .ok_or_else(|| AppError::Other("glTF missing meshes".into()))?;
    let mut imported = Vec::<ImportMesh>::new();

    if let Some(nodes) = document.get("nodes").and_then(|value| value.as_array()) {
        let roots = gltf_scene_roots(document)?;
        for node_index in roots {
            append_gltf_node(
                document,
                &buffer_views,
                &buffers,
                meshes,
                nodes,
                node_index,
                Mat4::identity(),
                "",
                fallback_name,
                scale,
                texture_resolver,
                &mut imported,
            )?;
        }
    } else {
        for mesh_index in 0..meshes.len() {
            append_gltf_mesh(
                document,
                &buffer_views,
                &buffers,
                meshes,
                mesh_index,
                None,
                Mat4::identity(),
                None,
                fallback_name,
                scale,
                texture_resolver,
                &mut imported,
            )?;
        }
    }

    if imported.is_empty() {
        return Err(AppError::Other(
            "glTF contained no triangle mesh primitives".into(),
        ));
    }
    Ok(imported)
}

fn gltf_scene_roots(document: &serde_json::Value) -> AppResult<Vec<usize>> {
    let scenes = document
        .get("scenes")
        .and_then(|value| value.as_array())
        .ok_or_else(|| AppError::Other("glTF missing scenes".into()))?;
    let scene_index = document
        .get("scene")
        .and_then(|value| value.as_u64())
        .unwrap_or(0) as usize;
    let scene = scenes
        .get(scene_index)
        .ok_or_else(|| AppError::Other(format!("glTF scene {scene_index} not found")))?;
    Ok(scene
        .get("nodes")
        .and_then(|value| value.as_array())
        .map(|nodes| {
            nodes
                .iter()
                .filter_map(|value| value.as_u64().map(|value| value as usize))
                .collect()
        })
        .unwrap_or_default())
}

#[allow(clippy::too_many_arguments)]
fn append_gltf_node(
    document: &serde_json::Value,
    buffer_views: &[GltfBufferView],
    buffers: &[Vec<u8>],
    meshes: &[serde_json::Value],
    nodes: &[serde_json::Value],
    node_index: usize,
    parent_transform: Mat4,
    parent_path: &str,
    fallback_name: &str,
    scale: f32,
    texture_resolver: &TextureResolver,
    imported: &mut Vec<ImportMesh>,
) -> AppResult<()> {
    let node = nodes
        .get(node_index)
        .ok_or_else(|| AppError::Other(format!("glTF node {node_index} not found")))?;
    let node_label = node
        .get("name")
        .and_then(|value| value.as_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| format!("node_{node_index}"));
    let node_path = if parent_path.is_empty() {
        node_label.clone()
    } else {
        format!("{parent_path}/{node_label}")
    };
    let transform = parent_transform.mul(gltf_node_transform(node)?);
    if let Some(mesh_index) = node.get("mesh").and_then(|value| value.as_u64()) {
        let node_name = node.get("name").and_then(|value| value.as_str());
        append_gltf_mesh(
            document,
            buffer_views,
            buffers,
            meshes,
            mesh_index as usize,
            node_name,
            transform,
            Some(&node_path),
            fallback_name,
            scale,
            texture_resolver,
            imported,
        )?;
    }

    if let Some(children) = node.get("children").and_then(|value| value.as_array()) {
        for child in children {
            if let Some(child_index) = child.as_u64() {
                append_gltf_node(
                    document,
                    buffer_views,
                    buffers,
                    meshes,
                    nodes,
                    child_index as usize,
                    transform,
                    &node_path,
                    fallback_name,
                    scale,
                    texture_resolver,
                    imported,
                )?;
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn append_gltf_mesh(
    document: &serde_json::Value,
    buffer_views: &[GltfBufferView],
    buffers: &[Vec<u8>],
    meshes: &[serde_json::Value],
    mesh_index: usize,
    node_name: Option<&str>,
    transform: Mat4,
    node_path: Option<&str>,
    fallback_name: &str,
    scale: f32,
    texture_resolver: &TextureResolver,
    imported: &mut Vec<ImportMesh>,
) -> AppResult<()> {
    let mesh = meshes
        .get(mesh_index)
        .ok_or_else(|| AppError::Other(format!("glTF mesh {mesh_index} not found")))?;
    let mesh_name = node_name
        .or_else(|| mesh.get("name").and_then(|value| value.as_str()))
        .unwrap_or(fallback_name);
    let Some(primitives) = mesh.get("primitives").and_then(|value| value.as_array()) else {
        return Ok(());
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
        let mut vertices =
            read_gltf_positions(document, buffer_views, buffers, position_accessor, scale)?;
        for vertex in &mut vertices {
            *vertex = transform.transform_point(*vertex);
        }
        let triangles = if let Some(index_accessor) =
            primitive.get("indices").and_then(|value| value.as_u64())
        {
            read_gltf_indices(document, buffer_views, buffers, index_accessor as usize)?
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
        let (material_name, color, texture_uri) =
            gltf_primitive_material(document, primitive, texture_resolver);
        imported.push(ImportMesh {
            name,
            vertices,
            triangles,
            material_name,
            texture_uri,
            color,
            hierarchy_path: node_path.map(|path| {
                if primitives.len() == 1 {
                    path.to_string()
                } else {
                    format!("{path}/{primitive_index}")
                }
            }),
            source_pivot: Some(transform.translation()),
        });
    }
    Ok(())
}

fn gltf_primitive_material(
    document: &serde_json::Value,
    primitive: &serde_json::Value,
    texture_resolver: &TextureResolver,
) -> (Option<String>, Option<[f32; 3]>, Option<String>) {
    let Some(material_index) = primitive.get("material").and_then(|value| value.as_u64()) else {
        return (None, None, None);
    };
    let Some(material) = document
        .get("materials")
        .and_then(|value| value.as_array())
        .and_then(|materials| materials.get(material_index as usize))
    else {
        return (None, None, None);
    };
    let material_name = material
        .get("name")
        .and_then(|value| value.as_str())
        .map(ToOwned::to_owned);
    let pbr = material.get("pbrMetallicRoughness");
    let color = pbr
        .and_then(|value| value.get("baseColorFactor"))
        .and_then(|value| value.as_array())
        .and_then(|values| {
            if values.len() >= 3 {
                Some([
                    values[0].as_f64()? as f32,
                    values[1].as_f64()? as f32,
                    values[2].as_f64()? as f32,
                ])
            } else {
                None
            }
        });
    let texture_uri = pbr
        .and_then(|value| value.get("baseColorTexture"))
        .and_then(|value| value.get("index"))
        .and_then(|value| value.as_u64())
        .and_then(|texture_index| gltf_texture_uri(document, texture_index as usize))
        .map(|uri| texture_resolver.resolve(&uri, Path::new(".")));
    (material_name, color, texture_uri)
}

fn gltf_texture_uri(document: &serde_json::Value, texture_index: usize) -> Option<String> {
    let source_index = document
        .get("textures")?
        .as_array()?
        .get(texture_index)?
        .get("source")?
        .as_u64()? as usize;
    document
        .get("images")?
        .as_array()?
        .get(source_index)?
        .get("uri")?
        .as_str()
        .map(ToOwned::to_owned)
}

fn gltf_node_transform(node: &serde_json::Value) -> AppResult<Mat4> {
    if let Some(matrix) = node.get("matrix") {
        let values = json_f32_array(matrix, 16, "node matrix")?;
        let mut out = [0.0f32; 16];
        out.copy_from_slice(&values);
        return Ok(Mat4(out));
    }

    let translation = node
        .get("translation")
        .map(|value| json_f32_array(value, 3, "node translation"))
        .transpose()?
        .unwrap_or_else(|| vec![0.0, 0.0, 0.0]);
    let rotation = node
        .get("rotation")
        .map(|value| json_f32_array(value, 4, "node rotation"))
        .transpose()?
        .unwrap_or_else(|| vec![0.0, 0.0, 0.0, 1.0]);
    let scale = node
        .get("scale")
        .map(|value| json_f32_array(value, 3, "node scale"))
        .transpose()?
        .unwrap_or_else(|| vec![1.0, 1.0, 1.0]);

    let [x, y, z, w] = normalize_quat([rotation[0], rotation[1], rotation[2], rotation[3]]);
    let xx = x * x;
    let yy = y * y;
    let zz = z * z;
    let xy = x * y;
    let xz = x * z;
    let yz = y * z;
    let wx = w * x;
    let wy = w * y;
    let wz = w * z;

    let mut matrix = [
        1.0 - 2.0 * (yy + zz),
        2.0 * (xy + wz),
        2.0 * (xz - wy),
        0.0,
        2.0 * (xy - wz),
        1.0 - 2.0 * (xx + zz),
        2.0 * (yz + wx),
        0.0,
        2.0 * (xz + wy),
        2.0 * (yz - wx),
        1.0 - 2.0 * (xx + yy),
        0.0,
        translation[0],
        translation[1],
        translation[2],
        1.0,
    ];
    for row in 0..3 {
        matrix[row] *= scale[0];
        matrix[4 + row] *= scale[1];
        matrix[8 + row] *= scale[2];
    }

    Ok(Mat4(matrix))
}

fn normalize_quat(mut quat: [f32; 4]) -> [f32; 4] {
    let len =
        (quat[0] * quat[0] + quat[1] * quat[1] + quat[2] * quat[2] + quat[3] * quat[3]).sqrt();
    if len > 0.0 {
        for value in &mut quat {
            *value /= len;
        }
    }
    quat
}

fn json_f32_array(value: &serde_json::Value, len: usize, context: &str) -> AppResult<Vec<f32>> {
    let array = value
        .as_array()
        .ok_or_else(|| AppError::Other(format!("{context} must be an array")))?;
    if array.len() != len {
        return Err(AppError::Other(format!(
            "{context} must contain {len} values"
        )));
    }
    array
        .iter()
        .map(|value| {
            value
                .as_f64()
                .map(|value| value as f32)
                .ok_or_else(|| AppError::Other(format!("{context} contains a non-number")))
        })
        .collect()
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
    use super::{parse_gltf_document, parse_obj, parse_stl, TextureResolver};

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
        let meshes = parse_gltf_document(
            &document,
            vec![buffer],
            "asset",
            1.0,
            &TextureResolver::default(),
        )
        .unwrap();
        assert_eq!(meshes.len(), 1);
        assert_eq!(meshes[0].name, "Tri");
        assert_eq!(meshes[0].vertices.len(), 3);
        assert_eq!(meshes[0].triangles, vec![[1, 2, 3]]);
    }

    #[test]
    fn gltf_document_applies_node_translation() {
        let mut buffer = Vec::new();
        for value in [0.0f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0] {
            buffer.extend_from_slice(&value.to_le_bytes());
        }
        let document = serde_json::json!({
            "buffers": [{ "byteLength": buffer.len() }],
            "bufferViews": [{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }],
            "accessors": [
                { "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" }
            ],
            "meshes": [{
                "name": "Tri",
                "primitives": [{ "attributes": { "POSITION": 0 } }]
            }],
            "nodes": [{ "name": "Moved", "mesh": 0, "translation": [2, 3, 4] }],
            "scenes": [{ "nodes": [0] }],
            "scene": 0
        });
        let meshes = parse_gltf_document(
            &document,
            vec![buffer],
            "asset",
            1.0,
            &TextureResolver::default(),
        )
        .unwrap();
        assert_eq!(meshes[0].name, "Moved");
        assert_eq!(meshes[0].vertices[0], [2.0, 3.0, 4.0]);
        assert_eq!(meshes[0].vertices[1], [3.0, 3.0, 4.0]);
    }
}
