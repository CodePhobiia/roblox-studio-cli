# Current Feature Expansion Plan

This plan keeps the product focused on depth, not breadth. The goal is to make the current command families robust across more Studio object types, asset types, reference shapes, failure modes, and proof artifacts before adding new top-level features.

Use the transfer fixes as the model: find one missing dimension, add serializer/deserializer/protocol/plugin coverage, add validation, add regression proof, and document the behavior.

## Operating Rules

- Do not add new command families unless an existing feature cannot be completed without one.
- Prefer expanding current commands through flags, validation rules, fixtures, and docs.
- Every expansion must define dry-run behavior, apply behavior, rollback or cleanup behavior, error shape, tests, and user-facing examples.
- Preserve live Studio sessions during validation. Do not kill bridge or Studio processes to make tests pass.
- Keep cloud side effects explicit. Uploads, rehosting, and ownership changes need clear warnings and artifact trails.

## Maturity Dimensions

Each current feature should be reviewed against these dimensions:

| Dimension | Question |
| --- | --- |
| Object coverage | Does it handle Parts, MeshParts, Tools, Models, UI, scripts, folders, attachments, bones, constraints, values, sounds, animations, and attributes? |
| Property coverage | Are class-specific and inherited properties serialized intentionally, including transforms, pivots, content IDs, physics, tags, attributes, and UI layout? |
| Reference coverage | Are object references preserved, rejected, or explained, including welds, Motor6Ds, constraints, ObjectValues, Adornees, PrimaryPart, Attachment refs, and external refs? |
| Asset ownership | Are source-owned and target-owned assets handled clearly, including rehost, unsupported delivery, private assets, missing permissions, and rollback limits? |
| Safety controls | Does the command have dry-run, explicit approval, overwrite rules, path ambiguity handling, ownership checks, and clear dangerous-operation labels? |
| Idempotency | Can the command be rerun without duplicating managed objects, losing user edits, or drifting metadata? |
| Proof | Does the command produce structured output, touched paths, warnings, rollback state, validation results, and test fixtures? |

## Feature Lanes

### Lane 1: Transfer, Package, Serialize, Deserialize

Purpose: make cross-Studio and package movement preserve complete gameplay objects, not just visible hierarchy.

Expand coverage for:

- Skinned MeshParts, `Bone`, `Attachment`, `Motor6D`, `Weld`, `WeldConstraint`, and constraint attachment refs.
- `Model.PrimaryPart`, pivots, tags, attributes, collection service tags, and package ownership metadata.
- UI trees with image properties, scrolling frame images, `SurfaceGui` and `BillboardGui` `Adornee` refs.
- Audio, animation, particle, trail, beam, decal, texture, and SurfaceAppearance content IDs.
- External references: block by default, allow explicitly, and report exact missing endpoint paths.
- Conflict modes: `fail`, `replace`, `merge`, and `rename` must behave consistently across transfer and package import.

Acceptance:

- Serializer fixtures cover each reference/property family.
- Deserializer preserves local transforms and refs after parenting.
- `transfer --dry-run`, `--replace`, `--rollback-on-error`, and `--rehost-images` have parity tests or smoke evidence.
- `docs/transfer-format.md` lists every supported dimension and known limitation.

### Lane 2: Import, Upload, Import-Uploaded, Rehost

Purpose: make asset ingestion reliable across content types and ownership boundaries.

Expand coverage for:

- Images used by UI, decals, textures, SurfaceAppearance, particles, trails, beams, and scrolling frames.
- Audio import/upload metadata, permission errors, unsupported local-only paths, and existing asset IDs.
- Mesh/model container imports with materials, texture URI metadata, pivots, hierarchy, and failure cleanup.
- Rehost flows with separate source and target credentials, delivery failures, upload wait timeouts, and partial rewrites.

Acceptance:

- Every asset command has size/type validation and a deterministic error for unsupported inputs.
- Rehost writes a manifest of source ID, target ID, property path, upload operation, and failure reason.
- Upload/profile errors never print API keys or token-like values.
- Docs show source-owned versus target-owned asset examples.

### Lane 3: Validate, Repair-Tool, Publish-Check, Deps

Purpose: make diagnostics catch real Studio breakage before a creator discovers it in Play Solo.

Expand coverage for:

- Broken welds, Motor6Ds, constraints, attachments, `ObjectValue`, `Adornee`, `PrimaryPart`, and external refs.
- Tool readiness: handle selection, anchored descendants, disconnected parts, collision/mass defaults, and articulated rigs.
- Skinned mesh readiness: bones present, bone transforms not identity-collapsed, mesh contains expected bone descendants.
- Asset readiness: private/missing image, audio, animation, mesh, particle, beam, trail, decal, and texture IDs.
- Ownership and publish readiness: unmanaged overwrite risk, cloud rollback limits, stale package checksums, and protocol drift.

Acceptance:

- `validate --fix` only applies diagnostics with explicit safe fix IDs.
- Repair reports before/after counts and never destroys valid articulated rigs by default.
- Publish/deps checks distinguish `fail`, `warn`, and `info` with stable rule IDs.
- Regression fixtures include at least one broken and one healthy case per rule family.

### Lane 4: Diff, Sync-Folder, Export, Upsert-Files, Apply-Plan

Purpose: make file and plan workflows reviewable, reversible, and safe for repeated use.

Expand coverage for:

- Script classes, ModuleScripts, LocalScripts, nested folders, UI images, mesh metadata, package manifests, and exported serialization blobs.
- Safe relative path handling, path collisions, invalid filenames, deleted files, renamed instances, and duplicate sibling names.
- Diff-to-fix-plan conversion with script exclusions, ownership filtering, dry-run/apply parity, and changed-path summaries.
- Sync pull/push metadata so Studio changes can be reconciled without clobbering manual edits.

Acceptance:

- `diff --fix-plan` emits plans that `apply-plan --dry-run` can explain without mutation.
- `sync-folder` and `export` share path validation rules.
- Upserts are idempotent for managed scripts and refuse unsafe unmanaged overwrites.
- Docs include one round-trip example from Studio to files and back.

### Lane 5: Bridge, Sessions, Auth, Protocol

Purpose: make the local bridge feel boring, recoverable, and safe in multi-Studio workflows.

Expand coverage for:

- Token lifecycle: auto-spawn, manual `bridge serve`, stale token recovery, env token override, and clear unauthorized repair instructions.
- Multi-session selection: exact name, UUID, ambiguous substring, protocol mismatch, stale heartbeat, and command timeout.
- Queue behavior under long imports, large payloads, canceled commands, bridge restart, and plugin reconnect.
- Endpoint auth consistency for every mutating command, with health/read-only behavior documented.

Acceptance:

- Unauthorized errors say which side is likely stale and list non-destructive recovery steps.
- Bridge tests cover registration, heartbeat expiry, token mismatch, protocol mismatch, queue result, and timeout.
- Long commands do not block unrelated Studio sessions.
- `doctor --fix` never interrupts existing sessions.

### Lane 6: Autopilot Existing Workflow

Purpose: improve proof quality without widening the Autopilot command surface.

Expand coverage for:

- Plan, preview, approval, live-gate, apply, rollback, proof, privacy, rehearsal, health, and closeout artifacts.
- Golden schemas for JSON outputs and stable markdown sections for review packets.
- Claim discipline: every closeout must separate planned, applied, validated, smoked, playtested, blocked, and unproven claims.
- Privacy scanner enforcement before handoff, model pack, review packet, or alpha evidence packet leaves the run directory.

Acceptance:

- No new Autopilot commands until current artifacts have golden tests.
- Existing commands produce enough evidence for `docs/alpha-evidence-demo-gate.md`.
- Modularization extracts pure helpers first and keeps behavior unchanged.
- One starter-shop run can be reviewed offline without implying live success.

### Lane 7: Smoke, Docs, Help, CI

Purpose: make the product explain and prove its current behavior.

Expand coverage for:

- Smoke fixtures for transfer, package, validate, repair-tool, import-ui-pack, upload mock, and rollback.
- CLI help examples that match real flags and avoid hidden required context.
- Docs that state current support, limitations, manual live steps, and rollback limits.
- CI gates for Rust fmt/test/build, Rojo build, plugin bundle drift, and protocol JSON round trips.

Acceptance:

- Each high-risk command has a minimal non-live test and a documented live smoke command.
- Generated plugin bundle drift is checked before release.
- Docs never claim broad public launch readiness from offline-only checks.
- Alpha evidence can be collected without relying on tribal knowledge.

## First Three Implementation Waves

### Wave 1: Portability Matrix

Focus on `serialize`, `deserialize`, `transfer`, and `package import/export`.

Tasks:

1. Build a property/reference matrix for classes currently supported by the plugin.
2. Add tests for missing inherited property families, starting with `Attachment`-derived classes and content-ID owners.
3. Add round-trip fixtures for welds, Motor6Ds, constraints, UI Adornees, skinned mesh bones, and package conflict modes.
4. Update `transfer-format.md` with supported and blocked dimensions.

### Wave 2: Reliability Matrix

Focus on `validate`, `repair-tool`, `publish-check`, `deps`, and bridge/session handling.

Tasks:

1. Convert transfer failure modes into validation rules.
2. Add repair previews for every safe automatic repair.
3. Improve unauthorized/token mismatch diagnostics without killing sessions.
4. Add smoke regression cases for stale refs, partial roots, rollback, and multi-Studio selection.

### Wave 3: Evidence Matrix

Focus on `diff`, `sync-folder`, `apply-plan`, `package`, `autopilot`, docs, and CI.

Tasks:

1. Ensure every mutating command has dry-run, touched paths, warnings, and rollback/cleanup reporting.
2. Add golden output tests for Autopilot proof artifacts and command JSON shapes.
3. Align help text and docs with actual flags.
4. Make CI enforce plugin drift and protocol compatibility before alpha tagging.

## Ticket Template

Use this template for every deepening task:

```text
Feature:
Dimension:
Current gap:
Expected behavior:
Dry-run behavior:
Apply behavior:
Rollback/cleanup:
Error shape:
Tests:
Docs/help updates:
Validation commands:
Residual risk:
```

## Definition Of Done

A current feature is alpha-solid only when:

- It handles the documented object/property/reference dimensions or rejects unsupported ones explicitly.
- It is idempotent for managed instances.
- It preserves user-owned content unless `force`, `replace`, or an equivalent explicit control is used.
- It emits structured output that can be used by agents and humans.
- It has focused tests for non-live behavior and a documented live Studio smoke path.
- Its docs describe both what works and what remains intentionally unsupported.
