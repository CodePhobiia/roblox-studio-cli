use crate::error::{AppError, AppResult};
use crate::protocol::messages::{
    DeserializeRequest, ExecRequest, ImportUiPackElement, ImportUiPackRequest,
    ImportUiPackResponse, RepairToolRequest, RepairToolResponse, SerializeRequest, ValidateRequest,
    ValidateResponse,
};
use serde::Serialize;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::time::Instant;

pub fn run(port: u16, studio: Option<String>, target: &str) -> AppResult<()> {
    match target {
        "validate" => smoke_validate(port, studio),
        "repair-tool" => smoke_repair_tool(port, studio),
        "import-ui-pack" => smoke_import_ui_pack(port, studio),
        "all" => {
            smoke_validate(port, studio.clone())?;
            smoke_repair_tool(port, studio.clone())?;
            smoke_import_ui_pack(port, studio)?;
            println!("All smoke checks passed.");
            Ok(())
        }
        other => Err(AppError::Other(format!("unknown smoke target: {other}"))),
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RegressionReport {
    ok: bool,
    steps: Vec<RegressionStep>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RegressionStep {
    name: String,
    ok: bool,
    duration_ms: u128,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

pub fn regression(
    port: u16,
    studio: Option<String>,
    out: Option<PathBuf>,
    upload_mock: bool,
) -> AppResult<()> {
    let mut steps = Vec::new();
    push_step(&mut steps, "validate", || {
        smoke_validate(port, studio.clone())
    });
    push_step(&mut steps, "repair-tool", || {
        smoke_repair_tool(port, studio.clone())
    });
    push_step(&mut steps, "import-ui-pack", || {
        smoke_import_ui_pack(port, studio.clone())
    });
    push_step(&mut steps, "weld-round-trip", || {
        smoke_weld_round_trip(port, studio.clone())
    });
    push_step(&mut steps, "package-conflict-dry-run", || {
        smoke_package_conflict(port, studio.clone())
    });
    push_step(&mut steps, "tool-equip-simulation", || {
        smoke_tool_equip(port, studio.clone())
    });
    if upload_mock {
        push_step_with_details(&mut steps, "upload-mock", smoke_upload_mock);
    }
    let ok = steps.iter().all(|step| step.ok);
    let report = RegressionReport { ok, steps };
    let output = serde_json::to_string_pretty(&report)?;
    if let Some(out) = out {
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&out, &output)?;
        println!("Regression report: {}", out.display());
    } else {
        println!("{output}");
    }
    if ok {
        Ok(())
    } else {
        Err(AppError::Other(
            "one or more smoke regression steps failed".into(),
        ))
    }
}

fn push_step<F>(steps: &mut Vec<RegressionStep>, name: &str, run: F)
where
    F: FnOnce() -> AppResult<()>,
{
    let started = Instant::now();
    match run() {
        Ok(()) => steps.push(RegressionStep {
            name: name.to_string(),
            ok: true,
            duration_ms: started.elapsed().as_millis(),
            details: None,
            error: None,
        }),
        Err(err) => steps.push(RegressionStep {
            name: name.to_string(),
            ok: false,
            duration_ms: started.elapsed().as_millis(),
            details: None,
            error: Some(err.to_string()),
        }),
    }
}

fn push_step_with_details<F>(steps: &mut Vec<RegressionStep>, name: &str, run: F)
where
    F: FnOnce() -> AppResult<Value>,
{
    let started = Instant::now();
    match run() {
        Ok(details) => steps.push(RegressionStep {
            name: name.to_string(),
            ok: true,
            duration_ms: started.elapsed().as_millis(),
            details: Some(details),
            error: None,
        }),
        Err(err) => steps.push(RegressionStep {
            name: name.to_string(),
            ok: false,
            duration_ms: started.elapsed().as_millis(),
            details: None,
            error: Some(err.to_string()),
        }),
    }
}

fn smoke_upload_mock() -> AppResult<Value> {
    let request = json!({
        "assetType": "Image",
        "displayName": "RsSmokeUploadMock",
        "description": "mock upload request with token sk-testvalue for redaction proof",
        "creationContext": {
            "creator": {
                "groupId": "123456"
            }
        }
    });
    let response = json!({
        "path": "operations/rs-smoke-upload-mock",
        "response": {
            "assetId": "987654321"
        },
        "apiKey": "sk-testvalue"
    });
    let file = json!({
        "field": "fileContent",
        "filename": "rs-smoke-upload.png",
        "mimeType": "image/png",
        "bytes": 4
    });
    let asset_id = response
        .get("response")
        .and_then(|value| value.get("assetId"))
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::Other("upload mock response missing assetId".into()))?;
    let redacted_request = redact_smoke_json(&request);
    let redacted_response = redact_smoke_json(&response);
    let serialized = serde_json::to_string(&json!({
        "request": redacted_request,
        "response": redacted_response,
        "file": file
    }))?;
    let mut checks = Vec::new();
    checks.push(upload_mock_check(
        "asset-type",
        request.get("assetType").and_then(Value::as_str) == Some("Image"),
    ));
    checks.push(upload_mock_check(
        "creator-shape",
        request
            .get("creationContext")
            .and_then(|value| value.get("creator"))
            .and_then(|value| value.get("groupId"))
            .and_then(Value::as_str)
            == Some("123456"),
    ));
    checks.push(upload_mock_check(
        "file-part-shape",
        file.get("field").and_then(Value::as_str) == Some("fileContent")
            && file.get("mimeType").and_then(Value::as_str) == Some("image/png"),
    ));
    checks.push(upload_mock_check("result-parsing", asset_id == "987654321"));
    checks.push(upload_mock_check(
        "redaction",
        !serialized.contains("sk-testvalue") && serialized.contains("<redacted>"),
    ));
    if checks
        .iter()
        .any(|check| check.get("ok").and_then(Value::as_bool) != Some(true))
    {
        return Err(AppError::Other(
            "upload mock request shaping or redaction check failed".into(),
        ));
    }
    Ok(json!({
        "mode": "offline",
        "request": redacted_request,
        "response": redacted_response,
        "file": file,
        "assetUri": format!("rbxassetid://{asset_id}"),
        "checks": checks
    }))
}

fn upload_mock_check(name: &str, ok: bool) -> Value {
    json!({ "name": name, "ok": ok })
}

fn redact_smoke_json(value: &Value) -> Value {
    match value {
        Value::String(text) => {
            if smoke_text_is_secretish(text) {
                Value::String("<redacted>".into())
            } else {
                Value::String(text.clone())
            }
        }
        Value::Array(items) => Value::Array(items.iter().map(redact_smoke_json).collect()),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(key, value)| {
                    if smoke_key_is_secretish(key) {
                        (key.clone(), Value::String("<redacted>".into()))
                    } else {
                        (key.clone(), redact_smoke_json(value))
                    }
                })
                .collect(),
        ),
        _ => value.clone(),
    }
}

fn smoke_key_is_secretish(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    ["api_key", "apikey", "token", "secret", "password"]
        .iter()
        .any(|marker| lower.contains(marker))
}

fn smoke_text_is_secretish(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("sk-")
        || lower.contains("api_key")
        || lower.contains("apikey")
        || lower.contains("token")
        || lower.contains("secret")
}

fn smoke_validate(port: u16, studio: Option<String>) -> AppResult<()> {
    cleanup(port, studio.clone(), "RsSmokeValidate")?;
    exec(
        port,
        studio.clone(),
        "local tool = Instance.new('Tool'); tool.Name = 'RsSmokeValidate'; local handle = Instance.new('Part'); handle.Name = 'Handle'; handle.Anchored = true; handle.Parent = tool; local blade = Instance.new('Part'); blade.Name = 'Blade'; blade.Parent = tool; local weld = Instance.new('Weld'); weld.Name = 'BrokenWeld'; weld.Parent = tool; tool.Parent = workspace; return tool:GetFullName()",
    )?;
    let result = validate(port, studio.clone(), "Workspace.RsSmokeValidate")?;
    cleanup(port, studio, "RsSmokeValidate")?;
    if result.summary.fail == 0 {
        return Err(AppError::Other(
            "smoke validate expected at least one failure".into(),
        ));
    }
    println!(
        "smoke validate passed: {} fail, {} warn, {} info",
        result.summary.fail, result.summary.warn, result.summary.info
    );
    Ok(())
}

fn smoke_repair_tool(port: u16, studio: Option<String>) -> AppResult<()> {
    cleanup(port, studio.clone(), "RsSmokeRepair")?;
    exec(
        port,
        studio.clone(),
        "local tool = Instance.new('Tool'); tool.Name = 'RsSmokeRepair'; local handle = Instance.new('Part'); handle.Name = 'Handle'; handle.Anchored = true; handle.Parent = tool; local blade = Instance.new('Part'); blade.Name = 'Blade'; blade.Parent = tool; tool.Parent = workspace; return tool:GetFullName()",
    )?;
    let repair: RepairToolResponse = crate::cli::request::post(
        port,
        "repair-tool",
        "/repair-tool",
        &RepairToolRequest {
            studio: studio.clone(),
            path: "Workspace.RsSmokeRepair".into(),
            handle: None,
            dry_run: false,
            replace_broken: true,
            physics_fix: true,
            collision: Some(false),
            massless: None,
        },
        75,
    )?;
    let result = validate(port, studio.clone(), "Workspace.RsSmokeRepair")?;
    cleanup(port, studio, "RsSmokeRepair")?;
    if repair.welds_created == 0 {
        return Err(AppError::Other(
            "smoke repair-tool expected at least one created weld".into(),
        ));
    }
    if result.summary.fail != 0 {
        return Err(AppError::Other(format!(
            "smoke repair-tool expected zero validation failures after repair, got {}",
            result.summary.fail
        )));
    }
    println!(
        "smoke repair-tool passed: {} weld(s), {} physics change(s)",
        repair.welds_created, repair.physics_properties_changed
    );
    Ok(())
}

fn smoke_import_ui_pack(port: u16, studio: Option<String>) -> AppResult<()> {
    exec(
        port,
        studio.clone(),
        "local gui = game:GetService('StarterGui'):FindFirstChild('RsSmokeUiPack'); if gui then gui:Destroy() end; return true",
    )?;
    let response: ImportUiPackResponse = crate::cli::request::post(
        port,
        "import-ui-pack",
        "/import-ui-pack",
        &ImportUiPackRequest {
            studio: studio.clone(),
            parent_path: "StarterGui".into(),
            name: "RsSmokeUiPack".into(),
            elements: vec![ImportUiPackElement {
                name: "SmokeIcon".into(),
                kind: "icon".into(),
                width: 1,
                height: 1,
                size_scale_x: 0.0,
                size_offset_x: 16,
                size_scale_y: 0.0,
                size_offset_y: 16,
                position_scale_x: 0.0,
                position_offset_x: 0,
                position_scale_y: 0.0,
                position_offset_y: 0,
                anchor_x: 0.0,
                anchor_y: 0.0,
                z_index: None,
                scale_type: Some("Fit".into()),
                background_transparency: Some(1.0),
                pixels_base64: "/wAA/w==".into(),
            }],
            source_id: None,
        },
        120,
    )?;
    exec(
        port,
        studio.clone(),
        "local gui = game:GetService('StarterGui'):FindFirstChild('RsSmokeUiPack'); assert(gui and gui:FindFirstChild('SmokeIcon'), 'SmokeIcon missing'); gui:Destroy(); return true",
    )?;
    if response.element_count != 1 {
        return Err(AppError::Other(format!(
            "smoke import-ui-pack expected one element, got {}",
            response.element_count
        )));
    }
    println!("smoke import-ui-pack passed: {}", response.gui_path);
    Ok(())
}

fn smoke_weld_round_trip(port: u16, studio: Option<String>) -> AppResult<()> {
    cleanup(port, studio.clone(), "RsSmokeWeldRoundTrip")?;
    cleanup(port, studio.clone(), "RsSmokeWeldRoundTrip_2")?;
    exec(
        port,
        studio.clone(),
        "local folder = Instance.new('Folder'); folder.Name = 'RsSmokeWeldRoundTrip'; local a = Instance.new('Part'); a.Name = 'A'; a.Parent = folder; local b = Instance.new('Part'); b.Name = 'B'; b.Parent = folder; local weld = Instance.new('Weld'); weld.Name = 'AB'; weld.Part0 = a; weld.Part1 = b; weld.Parent = folder; folder.Parent = workspace; return folder:GetFullName()",
    )?;
    let blob: serde_json::Value = crate::cli::request::post(
        port,
        "serialize",
        "/serialize",
        &SerializeRequest {
            studio: studio.clone(),
            path: "Workspace.RsSmokeWeldRoundTrip".into(),
        },
        75,
    )?;
    let _: serde_json::Value = crate::cli::request::post(
        port,
        "deserialize",
        "/deserialize",
        &DeserializeRequest {
            studio: studio.clone(),
            parent_path: "Workspace".into(),
            blob,
            conflict_mode: Some("rename".into()),
            dry_run: false,
            rollback_on_error: true,
            package_id: Some("rs-smoke".into()),
            validate_rules: Vec::new(),
            fail_on_validation_failure: false,
            fail_on_external_refs: false,
        },
        120,
    )?;
    exec(
        port,
        studio.clone(),
        "local clone = workspace:FindFirstChild('RsSmokeWeldRoundTrip_2'); assert(clone, 'clone missing'); local weld = clone:FindFirstChild('AB'); assert(weld and weld.Part0 and weld.Part1, 'weld refs missing'); clone:Destroy(); local orig = workspace:FindFirstChild('RsSmokeWeldRoundTrip'); if orig then orig:Destroy() end; return true",
    )?;
    println!("smoke weld-round-trip passed");
    Ok(())
}

fn smoke_package_conflict(port: u16, studio: Option<String>) -> AppResult<()> {
    exec(
        port,
        studio.clone(),
        "local folder = workspace:FindFirstChild('RsSmokePkg'); if not folder then folder = Instance.new('Folder'); folder.Name = 'RsSmokePkg'; folder.Parent = workspace end; return true",
    )?;
    let blob = serde_json::json!({
        "version": 1,
        "root": "i0",
        "instances": {
            "i0": {
                "className": "Folder",
                "properties": { "Name": "RsSmokePkg" }
            }
        }
    });
    let response: serde_json::Value = crate::cli::request::post(
        port,
        "deserialize",
        "/deserialize",
        &DeserializeRequest {
            studio: studio.clone(),
            parent_path: "Workspace".into(),
            blob,
            conflict_mode: Some("fail".into()),
            dry_run: true,
            rollback_on_error: false,
            package_id: Some("rs-smoke".into()),
            validate_rules: Vec::new(),
            fail_on_validation_failure: false,
            fail_on_external_refs: false,
        },
        75,
    )?;
    cleanup(port, studio, "RsSmokePkg")?;
    if response
        .get("conflict")
        .and_then(serde_json::Value::as_bool)
        != Some(true)
    {
        return Err(AppError::Other(
            "expected package conflict dry-run to report conflict".into(),
        ));
    }
    println!("smoke package-conflict-dry-run passed");
    Ok(())
}

fn smoke_tool_equip(port: u16, studio: Option<String>) -> AppResult<()> {
    cleanup(port, studio.clone(), "RsSmokeEquipTool")?;
    exec(
        port,
        studio.clone(),
        "local tool = Instance.new('Tool'); tool.Name = 'RsSmokeEquipTool'; local handle = Instance.new('Part'); handle.Name = 'Handle'; handle.Anchored = false; handle.Parent = tool; tool.Parent = workspace; local model = Instance.new('Model'); model.Name = 'RsSmokeCharacter'; local humanoid = Instance.new('Humanoid'); humanoid.Parent = model; model.Parent = workspace; humanoid:EquipTool(tool); task.wait(0.2); local ok = tool.Parent == model; model:Destroy(); if tool.Parent then tool:Destroy() end; assert(ok, 'Humanoid did not equip tool'); return true",
    )?;
    println!("smoke tool-equip-simulation passed");
    Ok(())
}

fn validate(port: u16, studio: Option<String>, path: &str) -> AppResult<ValidateResponse> {
    crate::cli::request::post(
        port,
        "validate",
        "/validate",
        &ValidateRequest {
            studio,
            path: path.into(),
            rules: Vec::new(),
        },
        75,
    )
}

fn exec(port: u16, studio: Option<String>, lua: &str) -> AppResult<serde_json::Value> {
    crate::cli::request::post(
        port,
        "exec",
        "/exec",
        &ExecRequest {
            studio,
            lua: lua.into(),
            allow_dangerous_exec: true,
        },
        75,
    )
}

fn cleanup(port: u16, studio: Option<String>, name: &str) -> AppResult<()> {
    exec(
        port,
        studio,
        &format!(
            "local inst = workspace:FindFirstChild({}); if inst then inst:Destroy() end; return true",
            serde_json::to_string(name)?
        ),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regression_report_schema_is_stable_json() {
        let report = RegressionReport {
            ok: true,
            steps: vec![RegressionStep {
                name: "upload-mock".into(),
                ok: true,
                duration_ms: 0,
                details: Some(json!({"mode": "offline"})),
                error: None,
            }],
        };
        let value = serde_json::to_value(report).unwrap();
        assert_eq!(value["ok"], true);
        assert_eq!(value["steps"][0]["durationMs"], 0);
        assert_eq!(value["steps"][0]["details"]["mode"], "offline");
        assert!(value["steps"][0].get("duration_ms").is_none());
    }

    #[test]
    fn upload_mock_validates_shape_and_redacts_without_network() {
        let details = smoke_upload_mock().unwrap();
        assert_eq!(details["mode"], "offline");
        assert_eq!(details["request"]["assetType"], "Image");
        assert_eq!(
            details["request"]["creationContext"]["creator"]["groupId"],
            "123456"
        );
        assert_eq!(details["assetUri"], "rbxassetid://987654321");
        assert!(details["checks"]
            .as_array()
            .unwrap()
            .iter()
            .all(|check| check["ok"] == true));
        let serialized = serde_json::to_string(&details).unwrap();
        assert!(!serialized.contains("sk-testvalue"));
        assert!(serialized.contains("<redacted>"));
    }
}
