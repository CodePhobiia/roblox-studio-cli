use super::messages::*;

#[test]
fn envelope_ok_serializes() {
    let env = Envelope::ok(serde_json::json!({"hello": "world"}));
    let s = serde_json::to_string(&env).unwrap();
    assert!(s.contains(r#""ok":true"#));
    assert!(s.contains(r#""hello":"world""#));
}

#[test]
fn envelope_err_serializes() {
    let env = Envelope::<()>::err("oops", "internal");
    let s = serde_json::to_string(&env).unwrap();
    assert!(s.contains(r#""ok":false"#));
    assert!(s.contains(r#""error":"oops""#));
    assert!(s.contains(r#""code":"internal""#));
}

#[test]
fn register_request_roundtrips() {
    let req = RegisterRequest {
        id: "abc-123".to_string(),
        name: "Snipe a Slime!".to_string(),
        place_file_path: Some("D:\\Snipe a Slime!\\thyab2221.rbxl".to_string()),
    };
    let s = serde_json::to_string(&req).unwrap();
    let back: RegisterRequest = serde_json::from_str(&s).unwrap();
    assert_eq!(back.id, "abc-123");
    assert_eq!(back.name, "Snipe a Slime!");
}

#[test]
fn export_response_roundtrips() {
    let response = ExportResponse {
        root_path: "ServerStorage.SniperSkins".to_string(),
        files: vec![ExportFile {
            path: "SniperSkins/0000_SniperSkins_Folder/instance.json".to_string(),
            kind: "metadata".to_string(),
            content: None,
            json: Some(serde_json::json!({"className": "Folder"})),
        }],
        warnings: vec![],
    };

    let s = serde_json::to_string(&response).unwrap();
    let back: ExportResponse = serde_json::from_str(&s).unwrap();
    assert_eq!(back.root_path, "ServerStorage.SniperSkins");
    assert_eq!(back.files.len(), 1);
    assert_eq!(back.files[0].kind, "metadata");
}

#[test]
fn import_asset_request_roundtrips() {
    let req = ImportAssetRequest {
        studio: Some("Snipe a Slime!".to_string()),
        parent_path: "Workspace".to_string(),
        name: "ImportedCube".to_string(),
        meshes: vec![ImportMesh {
            name: "Cube".to_string(),
            vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            triangles: vec![[1, 2, 3]],
        }],
        anchored: false,
        weld: true,
    };

    let s = serde_json::to_string(&req).unwrap();
    let back: ImportAssetRequest = serde_json::from_str(&s).unwrap();
    assert_eq!(back.parent_path, "Workspace");
    assert_eq!(back.meshes.len(), 1);
    assert_eq!(back.meshes[0].triangles[0], [1, 2, 3]);
}

#[test]
fn import_image_request_roundtrips() {
    let req = ImportImageRequest {
        studio: Some("Snipe a Slime!".to_string()),
        parent_path: "StarterGui".to_string(),
        name: "Icon".to_string(),
        kind: "button".to_string(),
        width: 2,
        height: 2,
        ui_width: 32,
        ui_height: 32,
        position_x: 4,
        position_y: 8,
        pixels_base64: "AAAA".to_string(),
    };

    let s = serde_json::to_string(&req).unwrap();
    let back: ImportImageRequest = serde_json::from_str(&s).unwrap();
    assert_eq!(back.parent_path, "StarterGui");
    assert_eq!(back.kind, "button");
    assert_eq!(back.ui_width, 32);
}
