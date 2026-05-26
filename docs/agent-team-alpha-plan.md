# Agent Team Alpha Plan

This board converts the product review into parallel workstreams for a private-alpha readiness push. All teams must preserve existing sessions, avoid live Studio mutation unless explicitly approved, and keep changes scoped to their owned files.

## Team A: Credential Hardening

Owner scope: `crates/rs/src/cli/auth.rs`, auth command wiring in `crates/rs/src/main.rs`, focused auth tests, and auth docs.

Goal: remove or clearly mitigate plaintext Open Cloud API key risk for private alpha.

Acceptance:
- `rs auth profile list` never prints secrets.
- profile add/list/remove/default behavior remains backward compatible.
- insecure storage is detectable through an auth/profile doctor path or equivalent warning.
- tests cover redaction and insecure-storage detection.

## Team B: CI & Release Proof

Owner scope: `.github/workflows/`, release/build scripts, CI docs, plugin bundle drift checks.

Goal: make every PR prove Rust formatting, tests, release build, Rojo plugin build, and generated bundle freshness.

Acceptance:
- GitHub Actions workflow runs `cargo fmt --check -p rs`, `cargo test -p rs`, `cargo build --release -p rs`, and Rojo build.
- plugin bundle drift is detected by CI.
- workflow avoids secrets and live Studio dependencies.

## Team C: Alpha Onboarding UX

Owner scope: `docs/getting-started-alpha.md`, README links, alpha-facing command copy.

Goal: create one narrow "what do I run first?" path around the proof-bound starter shop workflow.

Acceptance:
- guide has exactly three demos: connect Studio, generate a safe offline plan, apply only after approval and close out honestly.
- commands match existing CLI names and flags.
- docs do not imply publish readiness or live playtest success without proof.

## Team D: Autopilot Modularization

Owner scope: module map and first low-risk extraction under `crates/rs/src/cli/autopilot/` if feasible.

Goal: start reducing `autopilot.rs` risk without changing behavior.

Acceptance:
- produce a concrete extraction map.
- if editing code, extract only pure helpers with tests and preserve public behavior.
- avoid broad churn in command routing.

## Team E: Alpha Evidence & Demo Gate

Owner scope: alpha evidence checklist/docs and non-live verification helpers.

Goal: define the proof packet required before inviting private-alpha testers.

Acceptance:
- evidence checklist covers plan, preview, changed paths, validation, rollback, smoke, privacy, and closeout.
- live Studio demo steps are explicit but not executed automatically.
- blockers distinguish repo checks from live Studio checks.
