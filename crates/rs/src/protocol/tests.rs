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
        protocol_version: Some(PLUGIN_PROTOCOL_VERSION),
        plugin_version: Some("0.2.0".to_string()),
        capabilities: vec!["validate".to_string()],
    };
    let s = serde_json::to_string(&req).unwrap();
    let back: RegisterRequest = serde_json::from_str(&s).unwrap();
    assert_eq!(back.id, "abc-123");
    assert_eq!(back.name, "Snipe a Slime!");
    assert_eq!(back.protocol_version, Some(PLUGIN_PROTOCOL_VERSION));
}

#[test]
fn autopilot_review_request_roundtrips() {
    let req = AutopilotReviewRequest {
        studio: Some("Demo".to_string()),
        action: "set".to_string(),
        run: Some(serde_json::json!({
            "schemaVersion": "rs.autopilot.review.v1",
            "runId": "autopilot-123",
            "status": "preview"
        })),
    };

    let s = serde_json::to_string(&req).unwrap();
    let back: AutopilotReviewRequest = serde_json::from_str(&s).unwrap();
    assert_eq!(back.studio.as_deref(), Some("Demo"));
    assert_eq!(back.action, "set");
    assert_eq!(back.run.unwrap()["schemaVersion"], "rs.autopilot.review.v1");
}

#[test]
fn exec_request_requires_explicit_dangerous_approval() {
    let legacy: ExecRequest =
        serde_json::from_str(r#"{"studio":"Demo","lua":"return 1"}"#).unwrap();
    assert!(!legacy.allow_dangerous_exec);

    let approved = ExecRequest {
        studio: Some("Demo".into()),
        lua: "return 1".into(),
        allow_dangerous_exec: true,
    };
    let value = serde_json::to_value(&approved).unwrap();
    assert_eq!(value["allowDangerousExec"], true);
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
            material_name: Some("Plastic".to_string()),
            texture_uri: None,
            color: Some([1.0, 0.0, 0.0]),
            hierarchy_path: Some("Root/Cube".to_string()),
            source_pivot: Some([0.0, 0.0, 0.0]),
        }],
        anchored: false,
        weld: true,
        source_id: None,
    };

    let s = serde_json::to_string(&req).unwrap();
    let back: ImportAssetRequest = serde_json::from_str(&s).unwrap();
    assert_eq!(back.parent_path, "Workspace");
    assert_eq!(back.meshes.len(), 1);
    assert_eq!(back.meshes[0].triangles[0], [1, 2, 3]);
    assert_eq!(back.meshes[0].material_name.as_deref(), Some("Plastic"));
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
        source_id: None,
    };

    let s = serde_json::to_string(&req).unwrap();
    let back: ImportImageRequest = serde_json::from_str(&s).unwrap();
    assert_eq!(back.parent_path, "StarterGui");
    assert_eq!(back.kind, "button");
    assert_eq!(back.ui_width, 32);
}

#[test]
fn validate_response_roundtrips() {
    let response = ValidateResponse {
        path: "Workspace.Tool".to_string(),
        summary: ValidationSummary {
            fail: 1,
            warn: 1,
            info: 0,
        },
        diagnostics: vec![Diagnostic {
            severity: "fail".to_string(),
            rule: "ref.missing".to_string(),
            path: "Workspace.Tool.Handle.Weld".to_string(),
            property: Some("Part0".to_string()),
            message: "Part0 is nil".to_string(),
            fix_id: Some("repair-tool".to_string()),
        }],
        warnings: vec![],
    };

    let s = serde_json::to_string(&response).unwrap();
    let back: ValidateResponse = serde_json::from_str(&s).unwrap();
    assert_eq!(back.summary.fail, 1);
    assert_eq!(back.diagnostics[0].property.as_deref(), Some("Part0"));
}

#[test]
fn repair_tool_request_roundtrips() {
    let request = RepairToolRequest {
        studio: Some("Snipe a Slime!".to_string()),
        path: "Workspace.Rifle".to_string(),
        handle: Some("Handle".to_string()),
        dry_run: true,
        replace_broken: true,
        physics_fix: true,
        collision: Some(false),
        massless: Some(true),
    };

    let s = serde_json::to_string(&request).unwrap();
    let back: RepairToolRequest = serde_json::from_str(&s).unwrap();
    assert!(back.dry_run);
    assert_eq!(back.collision, Some(false));
    assert_eq!(back.massless, Some(true));
}

#[test]
fn snapshot_response_roundtrips() {
    let response = SnapshotResponse {
        root_path: "Workspace".to_string(),
        total_instances: 2,
        max_depth: 1,
        class_counts: [("Folder".to_string(), 1), ("Part".to_string(), 1)]
            .into_iter()
            .collect(),
        script_counts: Default::default(),
        tool_count: 0,
        mesh_part_count: 0,
        ui_count: 0,
        remote_count: 0,
        asset_references: vec![],
        duplicate_sibling_names: vec![],
        top_subtrees: vec![SnapshotSubtree {
            path: "Workspace".to_string(),
            count: 2,
        }],
        paths: vec![],
        warnings: vec![],
    };

    let s = serde_json::to_string(&response).unwrap();
    let back: SnapshotResponse = serde_json::from_str(&s).unwrap();
    assert_eq!(back.total_instances, 2);
    assert_eq!(back.class_counts["Part"], 1);
}

#[test]
fn create_instance_request_roundtrips() {
    let request = CreateInstanceRequest {
        studio: None,
        parent_path: "Workspace".to_string(),
        class_name: "Part".to_string(),
        name: "SpawnPad".to_string(),
        properties: vec![CreateProperty {
            name: "Anchored".to_string(),
            value: serde_json::json!(true),
        }],
    };

    let s = serde_json::to_string(&request).unwrap();
    let back: CreateInstanceRequest = serde_json::from_str(&s).unwrap();
    assert_eq!(back.class_name, "Part");
    assert_eq!(back.properties[0].name, "Anchored");
}

#[test]
fn import_ui_pack_request_roundtrips() {
    let request = ImportUiPackRequest {
        studio: None,
        parent_path: "StarterGui".to_string(),
        name: "ShopGui".to_string(),
        elements: vec![ImportUiPackElement {
            name: "BuyButton".to_string(),
            kind: "button".to_string(),
            width: 2,
            height: 2,
            size_scale_x: 0.0,
            size_offset_x: 160,
            size_scale_y: 0.0,
            size_offset_y: 48,
            position_scale_x: 0.5,
            position_offset_x: 0,
            position_scale_y: 0.8,
            position_offset_y: 0,
            anchor_x: 0.5,
            anchor_y: 0.5,
            z_index: Some(2),
            scale_type: Some("Fit".to_string()),
            background_transparency: Some(1.0),
            pixels_base64: "AAAA".to_string(),
        }],
        source_id: None,
    };

    let s = serde_json::to_string(&request).unwrap();
    let back: ImportUiPackRequest = serde_json::from_str(&s).unwrap();
    assert_eq!(back.elements[0].kind, "button");
    assert_eq!(back.elements[0].size_offset_x, 160);
}

#[test]
fn import_audio_request_roundtrips() {
    let request = ImportAudioRequest {
        studio: None,
        parent_path: "SoundService".to_string(),
        sounds: vec![ImportAudioSound {
            name: "Click".to_string(),
            asset_id: "rbxassetid://123".to_string(),
            volume: Some(0.5),
            playback_speed: None,
            looped: false,
        }],
        source_id: None,
    };

    let s = serde_json::to_string(&request).unwrap();
    let back: ImportAudioRequest = serde_json::from_str(&s).unwrap();
    assert_eq!(back.sounds[0].asset_id, "rbxassetid://123");
}

#[test]
fn import_uploaded_request_roundtrips() {
    let request = ImportUploadedRequest {
        studio: None,
        parent_path: "StarterGui".to_string(),
        kind: "image".to_string(),
        name: "Icon".to_string(),
        asset_id: "rbxassetid://123".to_string(),
        ui_kind: Some("image".to_string()),
        ui_width: Some(64),
        ui_height: Some(64),
        position_x: Some(0),
        position_y: Some(0),
        volume: None,
        playback_speed: None,
        looped: false,
        source_id: Some("src".to_string()),
    };
    let s = serde_json::to_string(&request).unwrap();
    let back: ImportUploadedRequest = serde_json::from_str(&s).unwrap();
    assert_eq!(back.asset_id, "rbxassetid://123");
    assert_eq!(back.ui_width, Some(64));
}

#[test]
fn upsert_files_request_roundtrips() {
    let request = UpsertFilesRequest {
        studio: None,
        parent_path: "ServerScriptService".to_string(),
        dry_run: false,
        delete: false,
        force: false,
        items: vec![UpsertFileItem {
            path: "Main.server.lua".to_string(),
            class_name: "Script".to_string(),
            name: "Main".to_string(),
            source: Some("print('ok')".to_string()),
            attributes: Default::default(),
        }],
        source_id: None,
    };

    let s = serde_json::to_string(&request).unwrap();
    let back: UpsertFilesRequest = serde_json::from_str(&s).unwrap();
    assert_eq!(back.items[0].class_name, "Script");
}

#[test]
fn apply_plan_request_roundtrips() {
    let request = ApplyPlanRequest {
        studio: None,
        root_path: "Workspace.Tool".to_string(),
        plan: serde_json::json!({"safeToApply": true, "changes": []}),
        dry_run: true,
        approved: false,
        force: false,
        only: vec!["added".to_string(), "modified".to_string()],
        exclude: vec!["Scripts".to_string()],
    };
    let s = serde_json::to_string(&request).unwrap();
    let back: ApplyPlanRequest = serde_json::from_str(&s).unwrap();
    assert_eq!(back.root_path, "Workspace.Tool");
    assert_eq!(back.only.len(), 2);
}

#[test]
fn package_update_request_roundtrips() {
    let request = PackageUpdateRequest {
        studio: None,
        parent_path: "Workspace".to_string(),
        blob: serde_json::json!({"version": 1}),
        package_id: "rspkg-123".to_string(),
        mode: "owned-only".to_string(),
        dry_run: true,
        force: false,
    };
    let s = serde_json::to_string(&request).unwrap();
    let back: PackageUpdateRequest = serde_json::from_str(&s).unwrap();
    assert_eq!(back.package_id, "rspkg-123");
    assert_eq!(back.mode, "owned-only");
}

#[test]
fn deps_response_roundtrips() {
    let response = DepsResponse {
        root_path: "Workspace.Tool".to_string(),
        dependencies: vec![DependencyReference {
            path: "Workspace.Tool.Handle".to_string(),
            class_name: "MeshPart".to_string(),
            property: "MeshId".to_string(),
            kind: "mesh".to_string(),
            value: "rbxassetid://123".to_string(),
            flags: vec!["privateRisk".to_string()],
        }],
        scripts: vec!["Workspace.Tool.Script".to_string()],
        remotes: vec![],
        unowned_instances: vec![],
        warnings: vec![],
    };
    let s = serde_json::to_string(&response).unwrap();
    let back: DepsResponse = serde_json::from_str(&s).unwrap();
    assert_eq!(back.dependencies[0].kind, "mesh");
}
