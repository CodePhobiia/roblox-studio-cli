# Alpha Evidence And Demo Gate

This gate defines the proof packet required before inviting private-alpha testers.
It is intentionally stricter than an offline review packet: repo checks can prove
the CLI and generated artifacts are coherent, but only manual live Studio checks
can prove a run was applied, smoked, rollbackable, and playable in a real place.

Do not execute the live Studio commands in this document from automation. A human
operator must choose the Studio place, confirm creator approval, run the live
steps manually, and attach the resulting artifacts.

## Packet Layout

Create one evidence root per candidate run:

```text
.rs\alpha-evidence\<run-id>\
  repo-checks.md
  live-demo.md
  blockers.md
  artifacts\
```

The packet may link to files under `.rs\autopilot\runs\<run-id>\`, but the
reviewer must be able to find each required artifact from the packet root. Use
relative paths when possible. Do not copy secrets, API keys, Roblox credentials,
local profile data, or private Studio paths into the packet.

## Invite Decision

Private-alpha invite status is `GO` only when every mandatory row below is
`PASS` or explicitly marked `N/A` with a reviewer-approved reason. Any unresolved
blocker in the repo-check or live-Studio categories keeps the decision at
`NO-GO`.

| Gate | Required evidence | Repo check | Live Studio check | Blocks invite when |
| --- | --- | --- | --- | --- |
| Plan | `plan.json`, generated file list, selected recipe or manifest, creator prompt, assumptions, and operation count. | `target\release\rs.exe autopilot plan --from-manifest examples\starter-shop.autopilot.json --out .rs\autopilot\runs\<run-id> --format json` or the exact prompt/manifest command used for the alpha run. | None. Planning is offline. | Plan is missing, schema-invalid, not reproducible, or includes unexplained risky/destructive operations. |
| Preview | `preview.json`, preview integrity state, operation summary, warnings, and safe-to-preview status. | `target\release\rs.exe autopilot preview --plan .rs\autopilot\runs\<run-id>\plan.json --format json` without `--live`. | Optional dry-run only: `target\release\rs.exe autopilot preview --studio "<Studio>" --plan .rs\autopilot\runs\<run-id>\plan.json --live --format json`. | Offline preview fails, generated files drift after preview, or live dry-run reports unresolved blockers. |
| Changed paths | A human-readable table of planned Studio paths, generated repo paths, and any repo docs/code changed for the alpha candidate. | Compare `plan.json`, `preview.json`, and `git diff --name-only` for the candidate branch. | Confirm `apply.json`, `history`, or Studio review output matches the same touched paths after live apply. | A path is unowned, unexpected, manually edited outside the packet, or not traceable to plan/apply evidence. |
| Validation | Exact static commands and results plus generated validation artifacts. | Minimum repo set: `cargo fmt --check -p rs`, `cargo test -p rs`, and `cargo build --release -p rs`. Add narrower command-specific tests when source code changed. | After live apply, require validation evidence from the approved apply command or a manual `rs validate` command recorded in `live-demo.md`. | Any required static check fails, validation is missing, or validation warnings are accepted without owner sign-off. |
| Rollback | `rollback.json`, `rollback.md`, rollback artifact path, restore command or manual restore note, and whether automatic restore is available. | `target\release\rs.exe autopilot rollback .rs\autopilot\runs\<run-id> --format json` may legitimately report `needsApply` before live work. | After apply, rerun rollback and verify the rollback artifact exists. If restore is manual, document the exact manual owner and restore path. | Applied changes have no rollback artifact, rollback scope is unclear, or restore requires unreviewed destructive action. |
| Smoke | `smoke-regression.json` or an explicit smoke waiver with owner and reason. | Before live work, record planned smoke coverage from `plan.json`, `rehearsal.json`, or `playtest-plan.json`. | Manual only: `target\release\rs.exe smoke regression --studio "<Studio>" --out .rs\autopilot\runs\<run-id>\smoke-regression.json --upload-mock`. | Regression smoke fails, is skipped without waiver, or does not cover the applied run. |
| Privacy | `privacy.json`, `privacy.md`, finding count, and reviewer notes for false positives. | `target\release\rs.exe autopilot privacy .rs\autopilot\runs\<run-id> --format json`. | Confirm live evidence files and screenshots do not include API keys, local paths, private user data, unpublished game secrets, or tester PII. | Any unredacted secret-like value, credential, private profile data, or tester PII remains in artifacts. |
| Closeout | `closeout.json`, `closeout.md`, safe-to-say claims, do-not-say claims, completion verdict, and next actions. | `target\release\rs.exe autopilot closeout .rs\autopilot\runs\<run-id> --format json`. Offline-only closeout should remain `needsLiveProof`. | After apply, smoke, rollback, and playtest evidence, closeout must be rerun and attached. | Closeout is missing, blocked, `needsLiveProof`, or contains claims broader than the evidence supports. |
| Blockers | `blockers.md` with every open issue, owner, severity, category, evidence path, and disposition. | Repo blockers include failed static checks, invalid plan/preview, missing artifacts, privacy findings, and unreviewed changed paths. | Live Studio blockers include bridge/plugin/protocol failure, missing approval, failed live preview/apply/validate/smoke, rollback gaps, and failed playtest evidence. | Any P0/P1 blocker is open, uncategorized, or lacks a dated owner decision. |

## Repo-Only Build Order

These steps are safe for a non-live evidence pass and must not contact Studio
unless the operator adds a `--studio`, `--live`, or live command explicitly.

```powershell
cargo fmt --check -p rs
cargo test -p rs
cargo build --release -p rs

target\release\rs.exe autopilot plan --from-manifest examples\starter-shop.autopilot.json --out .rs\autopilot\runs\<run-id> --format json
target\release\rs.exe autopilot preview --plan .rs\autopilot\runs\<run-id>\plan.json --format json
target\release\rs.exe autopilot review-pack .rs\autopilot\runs\<run-id> --format json
target\release\rs.exe autopilot proof .rs\autopilot\runs\<run-id> --format json
target\release\rs.exe autopilot privacy .rs\autopilot\runs\<run-id> --format json
target\release\rs.exe autopilot approval .rs\autopilot\runs\<run-id> --format json
target\release\rs.exe autopilot rehearsal .rs\autopilot\runs\<run-id> --format json
target\release\rs.exe autopilot closeout .rs\autopilot\runs\<run-id> --format json
```

Expected repo-only result: the packet can be `ready for manual live demo`, but
not `ready for private-alpha testers`. Offline artifacts must keep live apply,
smoke, rollback, and playtest proof as missing until a human runs the live steps.

## Manual Live Studio Demo Steps

Run these steps only from an operator-controlled terminal with the intended
alpha sandbox place open in Roblox Studio. Do not stop bridge or Studio session
processes while collecting evidence.

1. Record the place name, Studio version when visible, CLI commit, plugin bundle
   timestamp or hash when available, operator, date, and run directory in
   `live-demo.md`.
2. Check readiness without mutating the place:

   ```powershell
   target\release\rs.exe doctor --format json
   target\release\rs.exe list --json
   target\release\rs.exe autopilot preview --studio "<Studio>" --plan .rs\autopilot\runs\<run-id>\plan.json --live --format json
   ```

3. Present `review-pack.md`, `approval.md`, `rehearsal.md`, and changed paths to
   the owner. Record the exact approval wording. Approval must name the exact
   run, target Studio, and apply command.
4. Run the live gate. Continue only if the gate status is `readyToApply`:

   ```powershell
   target\release\rs.exe autopilot live-gate --run-dir .rs\autopilot\runs\<run-id> --approved --studio "<Studio>" --format json
   ```

5. Apply only the approved command from `live-gate.json` or `approval.json`.
   The command must include rollback capture and validation, for example:

   ```powershell
   target\release\rs.exe autopilot apply --studio "<Studio>" --plan .rs\autopilot\runs\<run-id>\plan.json --yes --rollback-on-error --validate --smoke regression --format json
   ```

6. Run regression smoke manually and attach the JSON report:

   ```powershell
   target\release\rs.exe smoke regression --studio "<Studio>" --out .rs\autopilot\runs\<run-id>\smoke-regression.json --upload-mock
   ```

7. Play the demo scenario from `playtest-plan.md` or `rehearsal.md`. Record the
   observed result with evidence paths, scenario results, and notes:

   ```powershell
   target\release\rs.exe autopilot record-playtest .rs\autopilot\runs\<run-id> --result passed --evidence "<screenshot-or-log-path>" --note "<human observation>" --format json
   ```

8. Refresh proof after the live evidence is attached:

   ```powershell
   target\release\rs.exe autopilot evidence-review .rs\autopilot\runs\<run-id> --format json
   target\release\rs.exe autopilot rollback .rs\autopilot\runs\<run-id> --format json
   target\release\rs.exe autopilot health .rs\autopilot\runs\<run-id> --format json
   target\release\rs.exe autopilot privacy .rs\autopilot\runs\<run-id> --format json
   target\release\rs.exe autopilot closeout .rs\autopilot\runs\<run-id> --format json
   ```

9. Write the final invite decision in `blockers.md`. If the result is not `GO`,
   list the smallest next proof step instead of broad remediation language.

## Blocker Format

Use this table in `blockers.md`:

| ID | Category | Severity | Evidence | Owner | Decision |
| --- | --- | --- | --- | --- | --- |
| `REPO-001` | Repo check | `P0` | `repo-checks.md` | `<name>` | Failed `cargo test -p rs`; invite blocked. |
| `LIVE-001` | Live Studio check | `P0` | `live-demo.md` | `<name>` | Regression smoke not run; invite blocked. |

Categories must be one of `Repo check`, `Live Studio check`, `Privacy`,
`Rollback`, `Smoke`, `Validation`, `Changed paths`, or `Closeout`.

## Claim Rules

Allowed before live work:

- "The repo evidence packet is ready for a manual live demo."
- "The offline plan and preview artifacts exist and are reviewable."
- "The run is not private-alpha ready until live apply, smoke, rollback,
  playtest, privacy, and closeout evidence pass."

Allowed after all gates pass:

- "The private-alpha proof packet is complete for `<run-id>`."
- "The run passed the recorded repo checks and manual live Studio gates listed
  in the packet."

Never claim:

- The feature is production-ready from offline evidence alone.
- Studio was changed unless `apply.json` and live demo notes prove it.
- Regression smoke passed unless `smoke-regression.json` proves it.
- Rollback is available unless `rollback.json` points to a real artifact or a
  reviewed manual restore path.
- Tester privacy is clean unless the latest `privacy.json` and live evidence
  review are attached.
