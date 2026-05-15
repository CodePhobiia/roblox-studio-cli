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
