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

1. Fix `/deserialize` payload parity so validation, external-ref, and rollback
   guards survive every bridge path.
2. Build a property/reference/content-ID matrix for classes currently supported
   by the plugin.
3. Align serializer, exporter, deps, validate, snapshot, and rehost coverage for
   image-like and content-like properties.
4. Add tests for missing inherited property families, starting with
   `Attachment`-derived classes, skinned mesh bones, and `SurfaceAppearance`
   maps.
5. Add round-trip fixtures for welds, Motor6Ds, constraints, UI Adornees,
   package conflict modes, and external-ref rejection.
6. Require package preflight verification before import/update mutation.
7. Update `transfer-format.md` with supported and blocked dimensions.

### Wave 2: Reliability Matrix

Focus on `validate`, `repair-tool`, `publish-check`, `deps`, bridge/session
handling, and import/upload idempotency.

Tasks:

1. Add a stable diagnostic/fix catalog and expose rule IDs in text and JSON
   output.
2. Convert transfer, package, asset, and sync failure modes into validation or
   publish-check rules.
3. Add repair previews for every safe automatic repair and make unsafe fixes
   explicitly non-fixable.
4. Improve unauthorized/token mismatch diagnostics without killing sessions.
5. Derive stable source IDs for local/imported assets so reruns are idempotent.
6. Add upload/rehost mock tests for permission, timeout, operation failure, and
   secret redaction paths.
7. Add smoke regression cases for stale refs, partial roots, rollback,
   validation fix, deps, publish-check, and multi-Studio selection.

### Wave 3: Evidence Matrix

Focus on `diff`, `sync-folder`, `apply-plan`, `package`, `autopilot`, docs, and CI.

Tasks:

1. Ensure every mutating command has dry-run, touched paths, warnings,
   per-operation results, and rollback/cleanup reporting.
2. Make `diff --fix-plan` and `apply-plan` share a stable operation schema.
3. Add source-ID ownership filters for apply/sync/package mutation.
4. Bind Autopilot approval, live gate, plan hash, preview integrity, and the
   exact approved apply command.
5. Add golden output tests for Autopilot proof artifacts and command JSON
   shapes.
6. Align help text and docs with actual flags.
7. Make CI enforce plugin drift, protocol compatibility, capability parity, and
   non-live docs/help checks before alpha tagging.

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

## Code-Backed Feature Backlog

This backlog expands each current feature family from the code that exists now.
Use it as the source for implementation tickets. It is intentionally scoped to
existing commands and handlers.

### Bridge, Doctor, Install-Plugin, Auth, Protocol

Code surfaces: `crates/rs/src/bridge/auth.rs`,
`crates/rs/src/bridge/auto_spawn.rs`, `crates/rs/src/bridge/registry.rs`,
`crates/rs/src/bridge/server.rs`, `crates/rs/src/cli/doctor.rs`,
`crates/rs/src/cli/install_plugin.rs`, `crates/rs/src/error.rs`,
`plugin/src/Main.server.lua`.

Expand:

- Add stale-token diagnosis for unauthorized bridge calls, including likely
  stale CLI token, stale bridge token, missing header, and wrong token file.
- Map `unauthorized` to a distinct remediation path and exit behavior.
- Document `RS_BRIDGE_TOKEN`, `RS_BRIDGE_TOKEN_FILE`, and safe recovery without
  stopping Studio sessions.
- Enforce plugin capabilities per command, not just protocol version.
- Add deterministic queue lifecycle checks for timeout cleanup, unknown result
  IDs, FIFO behavior, stale heartbeat expiry, and cross-session independence.
- Make `doctor --fix` report stable fix IDs and prove it does not call bridge
  shutdown or interrupt Studio sessions.
- Make `install-plugin` distinguish "no Studio restart needed" from "could not
  query bridge/session state."

Proof:

- Rust tests for token env/file precedence, auth middleware, protocol mismatch,
  capability mismatch, stale sessions, queue timeout cleanup, and doctor report
  builders.
- Static docs check for protocol/capability drift between Rust and
  `Main.server.lua`.

### List, Read, Exec

Code surfaces: `crates/rs/src/cli/list.rs`, `crates/rs/src/cli/read.rs`,
`crates/rs/src/cli/exec.rs`, `plugin/src/Handlers/Read.lua`,
`plugin/src/Handlers/Exec.lua`, `plugin/src/Serializer.lua`.

Expand:

- Keep `exec` explicitly dangerous and ensure every path requires the existing
  danger approval flag.
- Add capability/protocol refusal before dispatching `read` or `exec`.
- Make `read` output include warnings for skipped properties, external refs, and
  ambiguous paths when serializer evidence is available.
- Ensure text and JSON output expose the same error code and Studio session
  identity.

Proof:

- Protocol tests for read/exec payloads and auth behavior.
- Static help tests proving `exec` danger copy remains visible.

### Transfer

Code surfaces: `crates/rs/src/cli/transfer.rs`,
`crates/rs/src/bridge/orchestrator.rs`, `crates/rs/src/bridge/server.rs`,
`plugin/src/Handlers/Serialize.lua`, `plugin/src/Handlers/Deserialize.lua`,
`plugin/src/Serializer.lua`, `plugin/src/Deserializer.lua`.

Expand:

- Fix payload parity for `/deserialize`: forward `validateRules`,
  `failOnValidationFailure`, and `failOnExternalRefs` through every bridge path.
- Normalize conflict behavior with package import, either by adding
  `fail|replace|merge|rename` to transfer or documenting the intentional
  difference.
- Write a durable transfer proof object for dry-run and apply, including source
  Studio, target Studio, changed paths, external refs, validation result,
  rollback state, and rehost mapping.
- Keep external rigid refs blocked by default and report exact source paths and
  properties for each missing endpoint.
- Extend live smoke coverage for external-ref rejection, rollback-on-error, and
  rehosted transfer dry-run.

Proof:

- Bridge/protocol tests for deserialize guard forwarding.
- Smoke regression fixture for transfer of welds, Motor6Ds, constraints, bones,
  and partial roots.

### Serialize, Deserialize, Export, Package Format

Code surfaces: `plugin/src/PropertyAllowlist.lua`,
`plugin/src/PropertyEncoders.lua`, `plugin/src/Serializer.lua`,
`plugin/src/Deserializer.lua`, `plugin/src/Exporter.lua`,
`crates/rs/src/cli/export.rs`, `docs/transfer-format.md`.

Expand:

- Maintain a property/asset matrix shared by serializer, exporter, deps,
  validate, snapshot, and rehost code.
- Cover inherited property families explicitly, especially Attachment-derived
  classes, skinned mesh bones, constraints, content IDs, pivots, tags, and
  attributes.
- Add non-live golden blob fixtures for refs, `Model.PrimaryPart`, UI Adornees,
  `SurfaceGui`, `BillboardGui`, `SurfaceAppearance`, ParticleEmitter, Trail,
  Beam, Decal, Texture, Sound, Animation, and script Source.
- Document unsupported `Content`, `FontFace`, editable asset, rig weight, skin
  weight, UV, normal, and animation limitations honestly.

Proof:

- Static allowlist/export/deps/rehost parity test.
- Golden serializer/deserializer blob tests that catch dropped inherited
  transforms and dropped content properties.

### Package Export, Import, Update, Verify, Pack, Unpack

Code surfaces: `crates/rs/src/cli/package.rs`,
`plugin/src/Handlers/Deserialize.lua`, `plugin/src/Handlers/PackageUpdate.lua`,
`plugin/src/Ownership.lua`.

Expand:

- Require package preflight before mutation: manifest version, transfer blob
  readability, checksums, package ID, and known conflict mode.
- Deepen `package update --conflict-report` so it temp-deserializes and reports
  child-level owned/unowned conflicts, changed paths, refused paths, and
  rollback availability.
- Stamp and compare package manifest hash/checksum metadata in live Studio so
  `publish-check` can flag stale or orphaned managed installs.
- Align package asset refs with rehost and deps coverage; decide for each asset
  family whether import warns, rehosts, archives, or refuses.
- Add pack/unpack safe-path and checksum tests.

Proof:

- Unit tests for manifest/checksum/zip safety and legacy package refusal.
- Live smoke docs for update preserve/replace modes and package conflict
  reports.

### Import-Asset

Code surfaces: `crates/rs/src/cli/import_asset.rs`,
`plugin/src/Importer.lua`, `plugin/src/Handlers/ImportAsset.lua`.

Expand:

- Derive stable source IDs from file path plus content hash unless explicitly
  supplied, so repeated imports can update managed targets or report conflicts.
- Document mesh fidelity limits: no permanent cloud upload, no rig/skin weights,
  limited UV/normal guarantees, Blender fallback risks, and material/texture URI
  behavior.
- Add failure cleanup for partially imported mesh hierarchies and clearer errors
  for unsupported file types or Blender conversion failures.
- Add optional asset budget metadata in output without introducing a new
  command.

Proof:

- Static parser tests for mesh type handling and texture-root paths.
- Live smoke docs for ownership stamping, hierarchy, welds, and material
  metadata.

### Import-Image, Import-Ui-Pack

Code surfaces: `crates/rs/src/cli/import_image.rs`,
`crates/rs/src/cli/import_ui_pack.rs`, `plugin/src/ImportImage.lua`,
`plugin/src/Handlers/ImportImage.lua`, `plugin/src/Handlers/ImportUiPack.lua`.

Expand:

- Derive stable source IDs for local PNG imports and UI-pack elements.
- Make folder-mode `import-ui-pack` deterministic: either generate a sensible
  grid layout or document that a manifest is required for non-overlapping UI.
- Add idempotent reimport behavior by name/source ID, with conflict reporting
  for unmanaged existing GUI objects.
- Surface local EditableImage limits, downscale behavior, and `ImageContent`
  limitations in docs and command output.

Proof:

- Tests for PNG size/downscale metadata, manifest parsing, folder layout, and
  repeated import conflict behavior.
- Live smoke for import-ui-pack reimport and generated UI path reporting.

### Import-Audio, Upload, Import-Uploaded, Rehost

Code surfaces: `crates/rs/src/cli/import_audio.rs`,
`crates/rs/src/cli/import_uploaded.rs`, `crates/rs/src/cli/upload.rs`,
`crates/rs/src/cli/rehost_images.rs`, `plugin/src/Handlers/ImportAudio.lua`,
`plugin/src/Handlers/ImportUploaded.lua`.

Expand:

- Normalize and validate audio asset IDs in Rust before reaching Studio.
- Validate volume, playback speed, looped, and target class ranges.
- Add a mockable Open Cloud upload/delivery layer for permission errors,
  moderation failure, operation timeout, missing asset ID, and redaction.
- Extend rehost coverage to every image-like property already seen by deps and
  serializer, including `SurfaceAppearance` maps.
- Write a rehost manifest with source URI, source asset ID, class, property,
  Studio path, target URI, operation ID, and failure reason.
- Capture upload/import results as durable asset manifests for package and
  Autopilot evidence.

Proof:

- Unit tests for invalid audio IDs, upload wait errors, delivery failure bodies,
  and secret redaction.
- Mock-only smoke for upload/rehost request shaping; live upload remains
  explicit and credential-gated.

### Validate, Repair-Tool, Wire-Tool

Code surfaces: `crates/rs/src/cli/validate.rs`,
`crates/rs/src/cli/repair_tool.rs`, `plugin/src/Inspector.lua`,
`plugin/src/Handlers/Validate.lua`, `plugin/src/Handlers/RepairTool.lua`.

Expand:

- Add a stable diagnostic/fix catalog used by `Inspector.lua` and documented in
  `docs/diagnostics.md`.
- Show rule IDs and fix IDs in text output, not just JSON.
- Expand validate asset coverage to match deps: Decal, Texture, Animation,
  ParticleEmitter, Trail, Beam, SurfaceAppearance, Sound, MeshPart, UI images.
- Add rules for `Model.PrimaryPart`, skinned mesh bone readiness, script Source
  expectations, unsupported classes, duplicate paths, and package ownership
  drift.
- Split generic `repair-tool` fixes into precise safe fix IDs such as weld
  creation, physics defaults, and broken-joint removal.
- Keep missing-handle diagnostics non-fixable unless the user supplies a handle.
- Add repair changed paths and rollback/history proof.

Proof:

- One healthy and one broken fixture per rule family.
- Tests for `validate --fix` refusing diagnostics that are not explicitly
  safe-fixable.

### Deps, Publish-Check

Code surfaces: `crates/rs/src/cli/deps.rs`,
`crates/rs/src/cli/publish_check.rs`, `plugin/src/Handlers/Deps.lua`,
`crates/rs/src/cli/package.rs`.

Expand:

- Extract a structured publish-check report builder with stable check IDs,
  severity, source, path, property, and message.
- Preserve diagnostic IDs from validate/deps instead of collapsing them into
  strings.
- Compare live package metadata against local package manifests and checksums.
- Flag cloud rollback limits and private/unowned asset risks as publish-specific
  warnings.

Proof:

- Unit tests for report builder aggregation without Studio.
- Live smoke docs for deps and publish-check on healthy, missing asset, private
  risk, and stale package fixtures.

### Diff, Apply-Plan

Code surfaces: `crates/rs/src/cli/diff.rs`,
`crates/rs/src/cli/apply_plan.rs`, `plugin/src/Handlers/ApplyPlan.lua`,
`plugin/src/Ownership.lua`.

Expand:

- Harden identity beyond path-only matching: detect duplicate exported paths
  before map overwrite, represent renames/moves/class changes as explicit
  unsupported conflicts, and avoid silent subtree collapse.
- Make `operations` the stable plan schema consumed by `apply-plan`; keep
  `changes` as compatibility data if needed.
- Extend `ApplyPlanResponse` with per-operation results, rollback fields,
  refused paths, `rolledBack`, `snapshotRecorded`, and restore parent path.
- Add owned-only/source-ID filters so `rsManagedBy == "rs"` does not allow broad
  cross-source mutation.
- Prove dry-run/apply parity for scripts, tags, attributes, create/set/delete,
  and script exclusions.

Proof:

- Export-vs-export fixtures for added/deleted/modified/ref/duplicate/rename
  cases.
- Protocol tests for rollback fields and unsafe plan refusal.

### Sync-Folder, Sync Pull, Upsert-Files, Export

Code surfaces: `crates/rs/src/cli/sync_folder.rs`,
`crates/rs/src/cli/sync_pull.rs`, `crates/rs/src/cli/export.rs`,
`plugin/src/Handlers/UpsertFiles.lua`, `plugin/src/Handlers/Export.lua`.

Expand:

- Share one Rust safe path helper for export, sync pull, package, and generated
  artifact writes.
- Add plugin-side validation for `UpsertFiles` item paths: reject absolute
  paths, `..`, empty segments, reserved names, and ambiguous segments.
- Derive stable sync source IDs from manifest/folder roots unless supplied.
- Require matching source IDs for overwrite/delete unless `--force` is passed.
- Make PNG/mesh sync idempotent rather than create-only import flows.
- Have `sync pull` write a push-ready manifest mapping files back to classes,
  target parents, and source IDs.

Proof:

- Safe path table tests in Rust and static plugin validation fixtures.
- Repeat-sync tests for unchanged files, cross-source refusal, and delete-only
  owned `rsSyncPath` objects.

### Create, Snapshot, History, Transaction, Batch

Code surfaces: `crates/rs/src/cli/create.rs`,
`crates/rs/src/cli/snapshot.rs`, `crates/rs/src/cli/history.rs`,
`crates/rs/src/cli/transaction.rs`, `crates/rs/src/cli/batch.rs`,
`plugin/src/Handlers/Create.lua`, `plugin/src/Handlers/Snapshot.lua`,
`plugin/src/AuditLog.lua`.

Expand:

- Add `create --dry-run`, optional ownership stamping/source ID, duplicate
  policy (`fail|rename`), and undo metadata for created paths.
- Make snapshot output include enough metadata for rollback and review without
  implying publish readiness.
- Mark history entries as undoable or not undoable based on whether a handler
  emitted `snapshotBefore`.
- Expand batch manifests to orchestrate existing commands: export, diff,
  apply-plan, sync-folder, sync pull, package verify/update, transaction
  snapshot/restore, and history show.
- Document that batch is ordered and evidence-producing, not atomic.

Proof:

- Batch parser tests for every supported step type and dry-run propagation.
- History/undo smoke docs for handlers with and without rollback snapshots.

### Smoke Regression

Code surfaces: `crates/rs/src/cli/smoke.rs`.

Expand:

- Add smoke lanes for deps, publish-check, validate --fix, package ownership
  modes, stale package detection, rollback refusal, external refs, transfer
  dry-run/apply, and partial roots.
- Make `--upload-mock` validate request shaping, result parsing, and redaction
  instead of being a no-op.
- Keep all live Studio smoke commands manual and explicit; do not run them from
  CI or automated docs checks.

Proof:

- Non-live tests for smoke report schema.
- Manual live smoke commands in docs with expected artifact paths.

### Autopilot Current Workflow

Code surfaces: `crates/rs/src/cli/autopilot.rs`,
`crates/rs/src/cli/autopilot/util.rs`, `plugin/src/AutopilotReview.lua`,
`docs/alpha-evidence-demo-gate.md`, `docs/autopilot-modularization-map.md`.

Expand:

- Bind `approval.json`, `live-gate.json`, plan hash, preview integrity, and the
  exact apply command together so alpha/review apply paths cannot skip the gate.
- Make the alpha evidence gate machine-checkable through an existing command
  path such as `certify` or `review-pack`, rather than adding a new command.
- Add golden JSON/Markdown tests for review-pack, proof, approval, privacy,
  rollback, health, closeout, live-gate, and rehearsal.
- Enforce privacy scan before handoff, model pack, review pack, and evidence
  packet publication.
- Expand operation lowering parity for current commands, especially
  `import-ui-pack`, upload result manifests, package import proof, and smoke
  proof.
- Continue modularization in behavior-preserving slices: artifacts, operation
  filters, report renderers, recipe builders, privacy, proof, and claims.

Proof:

- Offline starter-shop run golden artifacts that never claim live success.
- Static tests that command guards reject live mutation, publish, upload, smoke,
  and bridge actions from offline-only agent loops.

### Docs, Help, CI

Code surfaces: `README.md`, `docs/*.md`, `plugin/README.md`,
`.github/workflows/ci.yml`.

Expand:

- Update stale plugin docs so they list current handlers and capabilities.
- Align docs with actual auto-spawn timeout and token behavior.
- Add help/docs parity checks for command examples and real flags.
- Add static protocol/capability parity checks against Rust and Luau.
- Keep CI non-live: Rust fmt/test/build, Rojo build, plugin drift, protocol
  round trips, docs/help checks, and static plugin scans.

Proof:

- CI job or static script that fails on stale plugin capability docs.
- Docs examples that state when a step is offline, live dry-run, live apply, or
  cloud side effect.

## Definition Of Done

A current feature is alpha-solid only when:

- It handles the documented object/property/reference dimensions or rejects
  unsupported ones explicitly.
- It is idempotent for managed instances.
- It preserves user-owned content unless `force`, `replace`, or an equivalent
  explicit control is used.
- It emits structured output that can be used by agents and humans.
- It has focused tests for non-live behavior and a documented live Studio smoke
  path.
- Its docs describe both what works and what remains intentionally unsupported.
