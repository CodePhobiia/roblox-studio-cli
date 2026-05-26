# Autopilot Modularization Map

Team D owns this map and the first low-risk helper extraction for `crates/rs/src/cli/autopilot.rs`.
The goal is to shrink the file without changing CLI behavior, command routing, live Studio flows, or
the plugin contract.

## Current Shape

`crates/rs/src/cli/autopilot.rs` is currently a single 129,217-line module. It mixes public command
option structs, JSON schemas, deterministic recipe builders, artifact I/O, offline report builders,
live readiness/apply flows, Markdown renderers, command guardrails, and unit tests.

Observed bands in the current file:

| Lines | Responsibility | Suggested destination |
| --- | --- | --- |
| 1-1,979 | imports, constants, public command option structs | `autopilot/options.rs` after call sites are stable |
| 1,980-9,029 | plan, report, recipe, readiness, and artifact DTOs | `autopilot/types.rs`, then split high-churn report DTOs by domain |
| 9,030-21,787 | public command entrypoints and orchestration shells | keep in root until internals move; later `autopilot/commands.rs` |
| 16,197-17,564 | setup/ready/live-gate readiness paths | `autopilot/readiness.rs`; keep bridge calls explicit |
| 19,213-23,170 | plan/preview/apply/context/survey/reconcile/scout/session and apply lowering | `autopilot/plan.rs`, `autopilot/preview.rs`, `autopilot/apply.rs`, `autopilot/survey.rs` |
| 23,171-40,474 | explain/coach/handoff/runs/mission/memory/preferences/game-bible/playbook/director/pursuit/control and creator-facing reports | `autopilot/reports/continuity.rs` and `autopilot/reports/creator.rs` |
| 40,475-45,187 | rollback, approval, privacy, next, opportunities, work-order/check, cycle, command guard | `autopilot/guards.rs` and `autopilot/reports/approval.rs` |
| 45,188-63,084 | runbook, flight recorder, navigator, model/task packs, best-friend/wow/demo flows | `autopilot/reports/agent_loop.rs` |
| 63,085-81,928 | roadmap, judgment, gameplay critique, plan analysis, source audit, planner/adopt/certify | `autopilot/analysis.rs` and `autopilot/certification.rs` |
| 81,929-93,327 | readiness/setup/apply helpers, recipe catalog, compose/tune builders, generated source materialization | `autopilot/recipes.rs`, `autopilot/artifacts.rs`, `autopilot/readiness.rs` |
| 93,328-110,583 | Markdown writers/renderers/summary printers | `autopilot/markdown.rs`, then report-specific modules as needed |
| 110,584-110,807 | pure JSON/path/text helpers | first extraction: `autopilot/util.rs` |
| 110,810-end | unit tests and fixtures | split tests with moved modules only when fixture dependencies are local |

## Extraction Order

1. `autopilot/util.rs`
   - Move pure helpers only: `slug`, `safe_join`, `redact_json`, and `redact_text`.
   - Keep public behavior identical and test these helpers without Studio.
   - This is the first landed extraction.

2. `autopilot/artifacts.rs`
   - Move `read_json_file`, `read_json_if_exists`, `write_json`, default run/manifest path helpers,
     and integrity/artifact path helpers.
   - Gate with existing plan/preview/adopt/bundle tests.
   - Do not change artifact names, schema versions, or default directories.

3. `autopilot/operation_filter.rs`
   - Move `to_lower_set`, `operation_group`, `operation_allowed`, `has_operation`, path join/relative
     helpers, script file naming, and apply-plan lowering filters.
   - Gate with the existing operation filter, asset grouping, delete guard, and apply refusal tests.

4. `autopilot/recipes.rs`
   - Move recipe catalog, manifest parsing, recipe inference, generated Luau builders, and compose/tune
     assembly.
   - Keep recipe aliases, generated file paths, operation order, and schema output byte-for-byte stable.

5. `autopilot/analysis.rs`
   - Move offline-only analyzers such as gameplay critique, source audit, plan impact, contracts,
     authority, UX, policy, and certification input readers.
   - These modules should depend on plan/artifact DTOs and utility helpers only.

6. `autopilot/readiness.rs` and `autopilot/apply.rs`
   - Move live readiness and apply flows last because they cross the bridge boundary.
   - Do not hide bridge errors, alter approval gates, or add fallback success states.
   - Keep `ready`, `setup`, `live-gate`, `preview --live`, and `apply` behavior covered by focused
     non-live tests plus documented manual live checks.

7. `autopilot/markdown.rs` and report modules
   - Move renderer/writer pairs in batches by report family after their builders are isolated.
   - Preserve headings, field names, next-command strings, and Markdown file names.

## Guardrails

- Do not edit `crates/rs/src/main.rs` command routing during helper extraction.
- Do not touch auth code, CI workflow files, alpha onboarding docs, plugin source, or generated plugin
  bundles from Team D work.
- Do not run live Studio commands or stop bridge/session processes during modularization.
- Prefer `pub(super)` for helper visibility until an external module genuinely needs wider access.
- Move tests with code only when fixtures do not pull the whole root module along; otherwise keep the
  existing root tests as regression coverage.
- Each extraction should run the smallest relevant `cargo test -p rs <test-name>` filters first, then a
  broader non-live Rust check that covers the touched module.

## Worker F Safety Additions

The Autopilot/Smoke/Evidence proof slice intentionally stayed in the root module
except for existing `autopilot/util.rs` helpers. The following helpers are good
future extraction candidates once `readiness.rs` and `apply.rs` are ready:

- Approval/live gate proof binding: plan hash, preview integrity, exact apply
  command, and pre-bridge `apply` refusal.
- Alpha packet completeness checks shared by `certify`, `review-pack`, and
  `evidence`.
- Privacy-scan enforcement before handoff, model-pack, review-pack, and
  evidence packet publication.
- Offline smoke upload mock request-shape and redaction validation.

Keep these moves behavior-preserving. The current safety contract depends on the
same command strings, JSON field names, Markdown headings, and refusal messages
remaining stable for downstream review packets.
