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
