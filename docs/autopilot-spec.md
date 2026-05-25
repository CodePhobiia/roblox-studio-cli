# rs Autopilot Feature Specification

## Summary

`rs autopilot` is the product's flagship feature: a verified, rollbackable Studio change agent for Roblox projects. It accepts a high-level creator request, inspects the live Studio data model, produces a structured change plan, previews the exact mutations, applies only approved safe operations, validates the result, and records rollback evidence.

The core promise is:

> Describe a Studio feature once. Get a reviewable, reproducible, validated patch instead of a pile of ad hoc manual edits.

This is not a replacement for Roblox Studio Assistant. The differentiator is production control: dry-runs, diffs, ownership metadata, validation, package snapshots, rollback, and durable run reports.

## Goals

- Turn natural-language or manifest-based feature requests into structured Studio change plans.
- Reuse existing `rs` primitives instead of adding one-off mutation paths.
- Make every proposed change inspectable before it touches Studio.
- Enforce explicit approval for mutations.
- Validate the result with existing `validate`, `snapshot`, `diff`, `smoke`, `package`, and `history` surfaces.
- Preserve rollback artifacts for every non-trivial mutation.
- Keep secrets, local credentials, and private profile data out of planner context and logs.

## Non-Goals

- Do not execute arbitrary Luau produced by a planner without structured review.
- Do not bypass the bridge/plugin protocol.
- Do not silently overwrite user-created Studio instances.
- Do not use broad destructive deletes as a normal planning strategy.
- Do not pretend cloud asset upload, moderation, or permission-sensitive operations succeeded unless the real Open Cloud operation completes.
- Do not require an AI provider for the first implementation milestone; deterministic plan files should be runnable first.

## User Experience

### Dry-Run First

```powershell
rs autopilot "Add a starter shop UI with purchase remotes and server-side validation" `
  --studio "My Game" `
  --scope game `
  --dry-run `
  --out .rs\autopilot\runs\shop
```

Expected output:

```text
Autopilot run: shop
Studio: My Game
Scope: game

Preflight:
  PASS bridge is healthy
  PASS plugin protocol is compatible
  PASS target scope resolved

Plan:
  18 create operation(s)
  4 script upsert operation(s)
  2 validation operation(s)
  0 destructive operation(s)

Artifacts:
  .rs\autopilot\runs\shop\context.json
  .rs\autopilot\runs\shop\plan.json
  .rs\autopilot\runs\shop\preview.json
  .rs\autopilot\runs\shop\report.md

No Studio changes were applied.
```

### Apply With Approval

```powershell
rs autopilot apply `
  --studio "My Game" `
  --plan .rs\autopilot\runs\shop\plan.json `
  --yes `
  --validate `
  --rollback-on-error
```

Expected output:

```text
Applied 22 operation(s)
Validation: PASS
Rollback snapshot: .rs\autopilot\runs\shop\rollback.rspkg
Report: .rs\autopilot\runs\shop\report.md
```

### One-Command Demo Mode

```powershell
rs autopilot "Create a collectible coin system with a test coin in Workspace" `
  --studio "Demo Place" `
  --scope game `
  --apply `
  --yes `
  --validate `
  --rollback-on-error
```

`--apply` is only shorthand for "plan, preview, apply, validate, report" and must still obey the same safety rules as separate planning and application.

## Existing Capabilities To Reuse

Autopilot should compose these existing or in-progress surfaces:

- `doctor` for bridge, plugin, and protocol readiness.
- `list` for Studio discovery.
- `read` and `snapshot` for scoped context collection.
- `validate` and `repair-tool` for safety checks and common fixups.
- `diff --fix-plan` for reviewable mutation plans.
- `apply-plan` for approved property, topology, and ownership-aware changes.
- `sync-folder` / `upsert-files` for script and local file updates.
- `import-image`, `import-ui-pack`, `import-asset`, and `import-uploaded` for assets.
- `upload` for explicit Open Cloud asset publishing.
- `package`, `package verify`, and `package import` for rollback and portable bundles.
- `history` / `undo` for restoring plugin-captured snapshots when available.
- `batch` for ordered orchestration of existing commands.

Autopilot should not duplicate these behaviors. If an operation can be expressed by an existing command, the plan should reference that command-shaped operation.

## High-Level Architecture

```text
User prompt or manifest
        |
        v
Autopilot CLI
        |
        +--> Preflight: doctor, list, protocol, scope resolve
        |
        +--> Context collector: snapshot, read, validate, optional package
        |
        +--> Planner: deterministic recipe or AI-backed strict JSON generation
        |
        +--> Plan validator: schema, safety, ownership, risk, idempotency
        |
        +--> Preview: dry-run operations, diff, report
        |
        +--> Apply: existing rs commands and /apply-plan
        |
        +--> Verify: validate, snapshot, optional smoke, package verify
        |
        +--> Artifacts: report, plan, logs, rollback snapshot
```

The CLI owns orchestration. The Studio plugin owns authoritative Studio mutations. The planner only proposes structured operations; it does not directly mutate Studio.

## Command Surface

### `rs autopilot recipes`

Lists built-in deterministic recipes with aliases, prompt hints, preconditions, generated files, and created Studio paths.

```powershell
rs autopilot recipes --format json
```

Current recipes cover `starterShop`, `collectibleCoin`, `questSystem`, `roundManager`, `inventorySystem`, `adminPanel`, `lobbyTeleport`, `obbyCheckpoint`, `tycoonCore`, `toolSystem`, `enemyEncounter`, `npcInteraction`, and `saveDataScaffold`.

### `rs autopilot capabilities`

Writes `capability-atlas.json` plus `capability-atlas.md`, an AI-readable map of verified Autopilot affordances. It combines the deterministic recipe catalog, key workflows, command purposes, expected artifacts, safe `act` handlers, supported plan operation kinds, required live plugin capabilities, examples, and safety boundaries.

```powershell
rs autopilot capabilities --format json
rs autopilot capabilities --root .rs\autopilot\runs --markdown .rs\autopilot\capability-atlas.md
```

Use this as the first tool-knowledge packet for a fresh or resumed AI. The atlas helps the model choose real commands and avoid invented capabilities, but it is not proof that a feature was built, applied, playtested, published, or production-ready.

### `rs autopilot tune`

Turns creator intent into an explicit `autopilotCompose` manifest before planning. The command infers the recipe stack, names generated systems from the prompt theme, sets economy knobs such as `currencyName`, `coinValue`, `dropValue`, `upgradePrice`, and writes both a JSON manifest and Markdown review.

```powershell
rs autopilot tune "make a fast candy tycoon with a shop" --smoke regression
rs autopilot tune "space combat obby" --recipe obbyCheckpoint --recipe enemyEncounter --out .rs\autopilot\manifests\space.autopilot-compose.json
```

The generated manifest uses recipe objects, for example `{"kind":"tycoonCore","name":"CandyTycoon","currencyName":"Sweets","dropValue":20,"upgradePrice":75}`. `rs autopilot compose --from-manifest <file>` must honor those per-recipe objects rather than falling back to generic recipe defaults.

### `rs autopilot compose`

Combines several deterministic recipes into one namespaced plan. This is the AI-agent path for building a starter game loop without applying isolated systems one at a time.

```powershell
rs autopilot compose --preset fullStarterGame --smoke regression --out .rs\autopilot\runs\starter-game
rs autopilot compose --from-manifest examples\full-starter-game.autopilot-compose.json
```

Generated scripts are stored under recipe-specific folders such as `generated/startershop/ShopServer.server.lua`, and the composed plan appends one final `validate`, `deps`, `publishCheck`, and optional `smoke` operation.

### `rs autopilot context`

Captures a redacted live Studio context bundle for AI planning. The bundle includes snapshot, validation, dependency graph, optional bounded read data, supported operation kinds, and the deterministic recipe catalog.

```powershell
rs autopilot context --studio "My Game" --path game --out .rs\autopilot\context --include-read
```

### `rs autopilot survey`

Writes an AI-readable place survey from live Studio inspection or an existing `context.json`. The report turns snapshot, validation, dependency, and optional read data into system signals, risk findings, safe mutation zones, suggested recipes, and exact next commands for planner-pack or start.

```powershell
rs autopilot survey --path game --include-paths --out .rs\autopilot\survey.json
rs autopilot survey --context .rs\autopilot\context\context.json --format json
```

Use this before planning against an existing place. It does not mutate Studio; it prevents an AI from proposing changes without knowing the current scripts, remotes, UI, assets, validation warnings, and ownership boundaries.

### `rs autopilot scout`

Combines a creator request with a survey or context bundle into the next AI build move. It writes `scout.json` plus `scout.md` with selected scope, prompt intent, place signals, selected recipes, safe zones, do-not-do rules, blockers/warnings, and exact next commands for `start`, `compose`, or `planner-pack`.

```powershell
rs autopilot scout "add a shop with save data" --survey .rs\autopilot\survey.json --format json
rs autopilot scout "add quests" --context .rs\autopilot\context\context.json --scope game
```

Use this after `survey` and before `start` when an AI needs to decide whether deterministic recipes are enough, whether a custom planner pack is needed, and what place boundaries must be preserved.

### `rs autopilot session`

Turns a scout packet, survey, context bundle, or creator request into a full offline AI work session. When scout evidence is ready, it writes `session.json` plus `session.md`, bootstraps `start.json`, and produces the same review-pack, evidence-kit, capsule, approval, proof, acceptance, privacy, control, and user-brief artifacts that an AI needs before asking for live approval.

```powershell
rs autopilot session "make a tycoon with shop and saves" --survey .rs\autopilot\survey.json --format json
rs autopilot session --scout .rs\autopilot\scout.json --run-dir .rs\autopilot\runs\tycoon
```

Use this as the AI's work-order command: it stays offline and non-mutating, but it converts intent plus place evidence into a reviewable run folder and a prioritized command queue for approval, readiness, apply, and proof collection.

### `rs autopilot pitch`

Writes a creator-facing option board before committing to one build direction. It ranks deterministic recipe stacks for the prompt, emits `pitch.json` plus `pitch.md`, explains why each direction fits, lists acceptance criteria, and provides exact `drive` and `kickoff` commands for each candidate.

```powershell
rs autopilot pitch "make a tycoon with shop and saves" --format json
rs autopilot pitch "make an adventure hub" --max-candidates 4 --markdown .rs\reviews\adventure-pitch.md
```

Use this before `drive` when the AI should offer creative options instead of assuming the first interpretation is the best one. `pitch` does not create run folders or mutate Studio; selecting a direction starts with that candidate's `driveCommand`.

### `rs autopilot storyboard`

Writes a player-facing experience brief for a prompt or run folder. It emits `storyboard.json` plus `storyboard.md` with the player promise, core loop beats, UI surfaces, gameplay systems, acceptance criteria, demo script, and proof expectations.

```powershell
rs autopilot storyboard "make a tycoon with shop and saves" --format json
rs autopilot storyboard --run-dir .rs\autopilot\runs\tycoon --markdown .rs\reviews\tycoon-storyboard.md
```

Use this when the AI needs to explain what the generated game will feel like before asking for approval. It is non-mutating and complements `pitch`: pitch chooses the direction; storyboard explains the selected experience in testable player terms.

### `rs autopilot proposal`

Writes one creator-facing proposal packet from a prompt. It combines a pitch board, the recommended storyboard, safe-to-say claims, do-not-say claims, alternatives, and exact next commands into `proposal.json` plus `proposal.md`. Companion `proposal-pitch.*` and `proposal-storyboard.*` files are written next to it.

```powershell
rs autopilot proposal "make a tycoon with shop and saves" --format json
rs autopilot proposal "make an adventure hub" --max-candidates 4 --markdown .rs\reviews\adventure-proposal.md
```

Use this as the AI's creator-review packet before running `drive`. It does not create candidate run folders or mutate Studio; it tells the AI what it may safely say and what it must not claim.

### `rs autopilot companion`

Writes the one-file AI companion packet for a fresh creator request. It combines `proposal.json` and `setup.json` into `companion.json` plus `companion.md`, then adds the agent brief, recommended candidate id, setup readiness, blockers, exact next actions, safe-to-say claims, and do-not-say claims.

```powershell
rs autopilot companion "make a tycoon with shop and saves" --format json
rs autopilot companion "make an adventure hub" --studio "Demo Place" --timeout 30 --markdown .rs\reviews\adventure-companion.md
rs autopilot companion "make a shop" --fix --format json
```

Use this as the first command an AI agent runs when it wants to help a Roblox creator end-to-end. It does not record creator choice, launch a selected run, or mutate Studio. With `--fix`, it may build/copy the local plugin bundle before readiness checks, but it still requires Studio restart evidence and a later `ready` gate before live apply.

### `rs autopilot select`

Records the creator's chosen proposal candidate before an AI drives the build. It reads `proposal.json`, defaults to the recommended candidate unless `--candidate` is passed, and writes `selection.json` plus `selection.md` with the selected drive command, kickoff command, safe-to-say claims, and forbidden claims.

```powershell
rs autopilot select .rs\autopilot\proposal.json --format json
rs autopilot select .rs\autopilot\proposal.json --candidate economy-loop
```

Use this after `proposal` and before `drive` when the creator has picked a direction. It is non-mutating and creates durable choice memory so the next AI turn does not rely on chat context.

### `rs autopilot launch`

Consumes `selection.json` and drives the chosen candidate through the existing safe offline orchestration path. It writes `launch.json` plus `launch.md`, creates or refreshes the selected run's `drive.json`, and stops at the same approval/readiness/proof boundary as `drive`. Its next actions are curated to the safe handoff commands: continue from the drive boundary, open approval, check live readiness, or refresh launch.

```powershell
rs autopilot launch .rs\autopilot\selection.json --format json
rs autopilot launch .rs\reviews\selection.json --assume --smoke regression
```

Use this when the creator has accepted a proposal and the AI should do the maximum safe local work without retyping a generated command. `launch` is still non-mutating: it does not run `apply`, does not require live Studio, and does not claim playtest or rollback proof.

### `rs autopilot drive`

Safely bootstraps or resumes an AI-led build until the live mutation boundary. It runs the offline startup path when the run has no plan, refreshes live-gate, closeout, and timeline artifacts, writes `drive.json` plus `drive.md`, and stops with a `resumeCommand` for approval, readiness, or proof instead of mutating Studio.

```powershell
rs autopilot drive "make a tycoon with shop and saves" --format json
rs autopilot drive --run-dir .rs\autopilot\runs\tycoon --format json
rs autopilot drive "add quests" --run-dir .rs\autopilot\runs\quests --assume
```

Use this as the default command for an AI agent that wants to do the maximum safe work in one pass. `drive` is non-mutating: it can prepare plans, review packets, approval prompts, proof ledgers, closeout, and timeline state, but it will not run `apply`.

### `rs autopilot live-gate`

Writes the final go/no-go packet before any live apply. It refreshes review-pack, approval, privacy, rollback, bundle verification, and optional live readiness, then emits `live-gate.json` plus `live-gate.md` with required checks, blockers, do-not-claim rules, and the approved apply command only when `--approved` and live readiness both pass.

```powershell
rs autopilot live-gate --run-dir .rs\autopilot\runs\tycoon --format json
rs autopilot live-gate --session .rs\autopilot\session.json --approved --timeout 90
rs autopilot live-gate --run-dir .rs\autopilot\runs\tycoon --approved --skip-ready
```

Use this immediately before crossing from offline artifacts into Studio mutation. Without `--approved`, it returns `needsApproval`; without a passing ready check, it returns `needsLiveReadiness` or `blocked`.

### `rs autopilot rehearsal`

Writes a live-demo rehearsal packet without mutating Studio. It refreshes showcase, evidence-kit, live-gate, and closeout artifacts, then emits `rehearsal.json` plus `rehearsal.md` with an ordered runbook, creator script, evidence to collect, stop conditions, safe claims, forbidden claims, and exact approval/readiness/live-gate/apply/proof commands.

```powershell
rs autopilot rehearsal .rs\autopilot\runs\tycoon --format json
rs autopilot rehearsal .rs\autopilot\runs\tycoon --markdown .rs\reviews\tycoon-rehearsal.md
```

Use this when an AI is about to present a generated Roblox slice and needs one proof-aware path from "show the creator" to "only apply if approved" to "record live evidence" to "close out honestly." The packet marks the apply step as mutating, preserves approval and live-gate stop conditions, and refuses to treat rehearsal as live proof.

### `rs autopilot closeout`

Writes the honest done/not-done verdict for a run. It refreshes proof, acceptance, judgment, review-pack, privacy, and rollback artifacts, then emits `closeout.json` plus `closeout.md` with completion checks, safe-to-say claims, do-not-say claims, blockers, warnings, and next actions.

```powershell
rs autopilot closeout .rs\autopilot\runs\tycoon --format json
rs autopilot closeout .rs\autopilot\runs\tycoon --markdown .rs\reviews\tycoon-closeout.md
```

Use this before telling the creator a request is handled. Offline-ready work returns `needsLiveProof`, not `complete`; only live apply, rollback, playtest, proof, judgment, and acceptance evidence can produce a completion verdict.

### `rs autopilot timeline`

Writes a black-box timeline for a run folder. It reads the known Autopilot packets in lifecycle order, emits `timeline.json` plus `timeline.md`, reports each artifact's presence and status, flags missing required packets, warns when proof or gate packets are older than newer apply/playtest/bundle evidence, and provides one safest `resumeCommand`.

```powershell
rs autopilot timeline .rs\autopilot\runs\tycoon --format json
rs autopilot timeline .rs\autopilot\runs\tycoon --markdown .rs\reviews\tycoon-timeline.md
```

Use this when an AI needs to resume a run without rereading every JSON file. The timeline is non-mutating; it is the run's flight recorder and handoff index, not proof that the request is complete.

### `rs autopilot run`

Creates deterministic plan artifacts and applies them in one approved command. `--yes` is mandatory because this command mutates Studio.

```powershell
rs autopilot run "Create a collectible coin system" --studio "Demo Place" --rollback-on-error --validate --yes
```

### `rs autopilot explain`

Reviews an existing plan without contacting Studio. This is the safest agent handoff format: it validates schema, summarizes risk and operations, checks referenced generated files, reports blockers, and prints recommended next commands.

```powershell
rs autopilot explain --plan .rs\autopilot\runs\shop\plan.json --format json
```

### `rs autopilot coach`

Reads a plan or run directory and recommends the next safe agent action. It reports blockers, missing artifacts, whether the next command mutates Studio, and whether live Studio connectivity is required.

```powershell
rs autopilot coach --run-dir .rs\autopilot\runs\starter-game --format json
rs autopilot coach --plan .rs\autopilot\runs\shop\plan.json
```

### `rs autopilot handoff`

Writes the single packet another AI agent or CI job should read before continuing a run. It creates a missing `bundle.json`, verifies existing bundle hashes, and writes `handoff.json` plus `handoff.md` with status, blockers, warnings, artifacts, and prioritized next commands.

```powershell
rs autopilot handoff .rs\autopilot\runs\starter-game --format json
rs autopilot handoff .rs\autopilot\runs\starter-game --markdown .rs\handoffs\starter-game.md
```

### `rs autopilot runs`

Indexes prior run folders so an agent can find the correct continuation point. It reports status, risk, operation count, bundle verification state, blockers, warnings, and one recommended next command per run.

```powershell
rs autopilot runs --root .rs\autopilot\runs --limit 10 --format json
```

### `rs autopilot mission`

Writes a project-level AI mission packet. It indexes recent runs, selects the active continuation point, maps an optional creator prompt to recipe recommendations, and writes `mission.json` plus `mission.md` with blockers, warnings, next actions, and safe commands.

```powershell
rs autopilot mission "Add quests, coins, inventory, and a shop" --format json
rs autopilot mission --root .rs\autopilot\runs --limit 5 --out .rs\autopilot\mission.json
```

### `rs autopilot memory`

Writes a compact project-memory ledger from prior Autopilot runs. It records the active run, known created Studio paths, generated files, inferred recipes, gameplay critique verdicts and gaps, certification state, blockers, warnings, and exact next actions so an AI agent can reload project state without rereading every artifact.

```powershell
rs autopilot memory --root .rs\autopilot\runs --limit 20 --format json
rs autopilot memory --out .rs\autopilot\project-memory.json --markdown .rs\autopilot\project-memory.md
```

Use this at the start and end of long AI-led build sessions to preserve continuity across agents and turns.

### `rs autopilot preferences`

Writes a durable creator-preference profile from recent run prompts, recipe history, `decisions.json`, proposal selections, `feedback.json`, and `demo-learn.json`. It emits `creator-preferences.json` plus `creator-preferences.md` with explicit constraints, rejected directions, learned demo preferences, feedback themes, prompt themes, recipe affinities, planning guidance, safe claims, forbidden claims, and refresh commands.

```powershell
rs autopilot preferences --root .rs\autopilot\runs --format json
rs autopilot preferences --root .rs\autopilot\runs --limit 50 --markdown .rs\autopilot\creator-preferences.md
```

Use this before a new AI planning session when the creator has accumulated taste, constraints, or repeated feedback across runs. The profile is planning guidance only; it never approves live apply, publishing, uploading, or completion claims.

### `rs autopilot game-bible`

Writes the cross-run game bible for an AI-led project. It reads project memory, creator preferences, architect/storyboard, style-guide, and world-blueprint artifacts, then emits `game-bible.json` plus `game-bible.md` with project title, player promise, core loop, genre, tone, canon rules, style rules, world rules, systems, continuity rules, proof contract, source runs, safe claims, and forbidden claims.

```powershell
rs autopilot game-bible --root .rs\autopilot\runs --format json
rs autopilot game-bible --root .rs\autopilot\runs --limit 50 --markdown .rs\autopilot\game-bible.md
```

Use this when the AI needs to preserve a coherent game identity across many runs and patches. The bible is canon guidance only; it does not prove implementation, approve live apply, or replace style/world/proof artifacts.

### `rs autopilot playbook`

Writes the project-level AI operating playbook. It refreshes project memory, creator preferences, and the game bible, reads available `retrospective.json` artifacts from recent runs, then emits `ai-playbook.json` plus `ai-playbook.md` with operating principles, a default workflow, learned lessons, claim guardrails, anti-patterns, source runs, and exact next commands.

```powershell
rs autopilot playbook --root .rs\autopilot\runs --format json
rs autopilot playbook --root .rs\autopilot\runs --limit 50 --markdown .rs\autopilot\ai-playbook.md
```

Use this when a fresh AI should inherit how to work on the project, not just what the project contains. The playbook is guidance only: it does not mutate Studio, approve live apply, prove playtests, or override newer creator decisions.

### `rs autopilot director`

Writes a canon-aware creative director packet for the next ambitious build slate. It refreshes the game bible and opportunity map, then emits `director.json` plus `director.md` with strategic themes, recommended build bets, canon fit, exact safe offline commands, expected artifacts, proof needs, risks, safe claims, and forbidden claims. Supported safe bets are surfaced as `rs autopilot pursue --bet ...` next actions.

```powershell
rs autopilot director "add pets and quests" --root .rs\autopilot\runs --format json
rs autopilot director --root .rs\autopilot\runs --limit 50 --markdown .rs\autopilot\director.md
```

Use this when an AI needs to choose what to build next without drifting from project canon. The director ranks strategy only; it does not implement the bet, approve live apply, or replace creator decisions, alignment, live-gate, rollback, or playtest proof.

### `rs autopilot pursue`

Executes a selected creative director bet through safe offline handlers. It refreshes `director.json`, selects `--bet` or the first supported non-mutating bet, then writes `pursuit.json` plus `pursuit.md`. Unsupported, live-Studio, or mutating bets become explicit blockers; there is no arbitrary shell fallback.

```powershell
rs autopilot pursue "add pets and quests" --root .rs\autopilot\runs --format json
rs autopilot pursue --bet BET-002 --root .rs\autopilot\runs --dry-run --format json
```

Use this when the AI should move from strategy to the next offline run without hand-copying a director command. The first supported execution path is `kickoff`; live apply, creator approval, and playtest proof remain separate.

### `rs autopilot agenda`

Writes the AI's current work agenda. It refreshes a cockpit snapshot under `agenda-context/`, then distills the command queue into `agenda.json` plus `agenda.md` with prioritized work items, source, phase, exact command, expected artifacts, done-when checks, stop conditions, readiness flags, safe claims, and forbidden claims.

```powershell
rs autopilot agenda "add pets and quests" --root .rs\autopilot\runs --format json
rs autopilot agenda --run-dir .rs\autopilot\runs\shop --markdown .rs\reviews\shop-agenda.md
```

Use this when the AI needs a durable, claim-safe responsibility list rather than a broad dashboard. The agenda never mutates Studio and does not mark work complete; it tells the AI what to do first and what evidence must exist before reporting progress.

### `rs autopilot sprint`

Runs a bounded agenda sprint. It refreshes agenda evidence, selects the next agenda item with a supported internal offline handler, executes it through `act`, skips commands already attempted in the same sprint, and writes `sprint.json` plus `sprint.md`. It stops at dry-run, blocker, live/mutating boundary, unsupported agenda action, or `--max-steps`.

```powershell
rs autopilot sprint "add pets and quests" --run-dir .rs\autopilot\runs\shop --max-steps 3 --format json
rs autopilot sprint --root .rs\autopilot\runs --dry-run --format json
```

Use this when the AI should make several safe offline agenda moves without manually alternating between `agenda` and `act`. The sprint never shells out, never applies to Studio, never uploads or publishes assets, and never treats offline artifacts as live playtest or production proof.

### `rs autopilot retrospect`

Writes the AI work retrospective for a run. It reads recent evidence such as `sprint.json`, `agenda.json`, `act.json`, `loop.json`, `cycle.json`, `diagnosis.json`, `proof.json`, `acceptance.json`, `privacy.json`, `health.json`, `journal.json`, and `bundle.json`, then emits `retrospective.json` plus `retrospective.md` with accomplishments, lessons, blockers, warnings, safe claims, forbidden claims, and exact next commands.

```powershell
rs autopilot retrospect .rs\autopilot\runs\shop --format json
rs autopilot retrospect .rs\autopilot\runs\shop --markdown .rs\reviews\shop-retro.md
```

Use this after `sprint`, `loop`, `act`, or a long AI session. It gives the next model a compact "what happened and what we learned" packet without treating notes, sprint selection, or offline artifacts as live apply, playtest, rollback, publish, or production proof.

### `rs autopilot control`

Writes the single mission-control packet an AI should read before acting. It combines project memory, next move, roadmap, judgment, and repair-plan evidence into `control.json` plus `control.md`, then lists the recommended command, a short command queue, trusted artifact paths, blockers, warnings, and explicit do-not-claim guardrails.

```powershell
rs autopilot control --root .rs\autopilot\runs --format json
rs autopilot control "add quests and a shop" --root .rs\autopilot\runs --out .rs\autopilot\control.json
rs autopilot control --run-dir .rs\autopilot\runs\starter-game --format json
```

Use this at the start of every AI session. It does not mutate Studio; it prevents the agent from guessing which lower-level packet matters most.

### `rs autopilot brief`

Writes the creator-safe status update an AI can use when reporting progress. It produces `user-brief.json` plus `user-brief.md` with one plain-language update, `safeToSay`, `doNotSay`, supporting evidence, blockers, warnings, and next actions.

```powershell
rs autopilot brief --root .rs\autopilot\runs --format json
rs autopilot brief --run-dir .rs\autopilot\runs\starter-game
rs autopilot brief "add quests and a shop" --root .rs\autopilot\runs --out .rs\autopilot\user-brief.json
```

Use this before writing a user-facing summary. The command is deliberately conservative: missing live apply, rollback, bundle, or playtest proof becomes a forbidden claim instead of an overstated success report.

### `rs autopilot inbox`

Routes a raw creator message into the safest next Autopilot move. It writes `inbox.json` plus `inbox.md` with a redacted message, classified intent, selected run context, route commands, expected artifacts, safe-to-say claims, do-not-say guardrails, and whether any route needs live Studio.

```powershell
rs autopilot inbox "shop button is confusing" --run-dir .rs\autopilot\runs\starter-game --format json
rs autopilot inbox "go ahead and apply this" --run-dir .rs\autopilot\runs\starter-game --format json
rs autopilot inbox "add quests and a shop" --root .rs\autopilot\runs --format json
```

Use this before an AI interprets chat as instructions. Feedback routes to `feedback`, constraints route to `decision`, rollback asks route to `rollback`, status asks route to `cycle`/`respond`, new build asks route to `start`, and apply/publish/upload wording is held at approval/live-gate readiness instead of becoming mutation permission.

### `rs autopilot handle`

Handles one creator message through the safe offline route. It writes `handle.json` plus `handle.md`, refreshes `inbox.json`, executes exactly one supported non-mutating route such as feedback triage, decision recording, rollback packet, cycle/response refresh, mission refresh, or offline start, then records execution status, artifacts, safe claims, forbidden claims, and next actions.

```powershell
rs autopilot handle "shop button is confusing" --run-dir .rs\autopilot\runs\starter-game --format json
rs autopilot handle "go ahead and apply this" --run-dir .rs\autopilot\runs\starter-game --format json
rs autopilot handle "add quests and a shop" --root .rs\autopilot\runs --assume --format json
```

Use this when the AI should act on ordinary creator chat without shelling arbitrary commands. It does not execute approval, live-gate, ready, apply, upload, publish, bridge, smoke, setup, or plugin repair routes; those remain explicit gated follow-ups.

### `rs autopilot conversation`

Writes durable conversation state for a run. It emits `conversation.json` plus `conversation.md` by reading creator-facing and AI-facing artifacts such as inbox, handle, feedback, decisions, response, journal, loop, and cycle, then summarizes turns, open loops, safe-to-say claims, do-not-say guardrails, trusted artifacts, and next commands.

```powershell
rs autopilot conversation --run-dir .rs\autopilot\runs\starter-game --format json
rs autopilot conversation "make the shop clearer" --run-dir .rs\autopilot\runs\starter-game --format json
rs autopilot conversation "add quests and a shop" --root .rs\autopilot\runs --format json
```

Use this at the start of a resumed AI turn or before replying to the creator. It does not mutate Studio and does not claim any open feedback, approval, rollback, response, playtest, or publish work is complete unless the corresponding artifact supports it.

### `rs autopilot chat`

Handles one creator message through the full safe offline chat path. It writes `chat.json` plus `chat.md`, refreshes `handle.json`, creates feedback patch work orders when feedback was routed, runs bounded offline loop steps when appropriate, refreshes `conversation.json`, and writes a checked `response.json` when claim evidence supports a reply.

```powershell
rs autopilot chat "shop button is confusing" --run-dir .rs\autopilot\runs\starter-game --format json
rs autopilot chat "go ahead and apply this" --run-dir .rs\autopilot\runs\starter-game --format json
rs autopilot chat "add quests and a shop" --root .rs\autopilot\runs --assume --max-steps 2 --format json
```

Use this when an AI should safely handle ordinary creator chat end to end. It never treats chat wording as live apply permission; approval, live-gate, upload, publish, setup, smoke, and plugin repair remain explicit gated follow-ups.

### `rs autopilot intake`

Turns a creator request into the first AI-safe interpretation packet. It writes `intake.json` plus `intake.md` with interpreted intent, confidence, recipe stack, assumptions, clarifying questions, acceptance criteria, continuity warnings, and the safest first commands.

```powershell
rs autopilot intake "make a tycoon with a shop" --format json
rs autopilot intake "make it better" --root .rs\autopilot\runs --markdown .rs\autopilot\intake.md
rs autopilot intake --out .rs\autopilot\intake.json
```

Use this before `architect` when the AI needs to decide whether to ask a clarifying question or proceed with explicit assumptions. The command does not mutate Studio and does not require the bridge.

### `rs autopilot start`

Runs the offline AI startup flow behind one command. It writes `start.json` plus `start.md`, always writes intake artifacts, pauses on blocking clarification unless `--assume` is passed, and when safe creates kickoff, mission-control, user-brief, review-pack, evidence-kit, capsule, approval, proof, acceptance, and privacy artifacts for the selected run.

```powershell
rs autopilot start "make a tycoon with a shop" --format json
rs autopilot start "make it better" --root .rs\autopilot\runs
rs autopilot start "make it better" --assume --run-dir .rs\autopilot\runs\better-v1
```

Use this as the first command in an AI-led build session. It stays offline and non-mutating; live work still begins only after `ready`, explicit approval, rollback, validation, and playtest proof.

### `rs autopilot cockpit`

Writes the single dashboard an AI agent should keep open while building. It refreshes memory, next, roadmap, mission-control, creator-brief, proof-ledger, acceptance, privacy, decisions, alignment, and journal artifacts, then emits `cockpit.json` plus `cockpit.md` with status, selected run, readiness flags, panels, claim guardrails, evidence, artifacts, and the exact command queue.

```powershell
rs autopilot cockpit --root .rs\autopilot\runs --format json
rs autopilot cockpit --run-dir .rs\autopilot\runs\tycoon
rs autopilot cockpit "add quests and a shop" --out .rs\autopilot\cockpit.json
```

Use this at the start of every resumed AI session or after any new artifact is written. The command stays offline and non-mutating; it is meant to stop the model from juggling stale JSON files, forgetting creator decisions, or reporting claims that `user-brief.json` forbids.

### `rs autopilot capsule`

Writes the copy/paste-safe continuation packet for another AI session. It refreshes `cockpit.json`, proof, acceptance, user brief, privacy, decisions, alignment, and journal artifacts, then emits `agent-capsule.json` plus `agent-capsule.md` with required context files, a resume prompt, command queue, safe-to-say claims, do-not-say claims, blockers, warnings, and privacy handoff status.

```powershell
rs autopilot capsule --run-dir .rs\autopilot\runs\tycoon --format json
rs autopilot capsule "add quests and a shop" --root .rs\autopilot\runs
```

Use this when the current AI is about to hand work to another model or a fresh context window. The command stays offline, blocks clean handoff if the privacy scan finds unredacted secret-like values, and marks `decisions.json`, `alignment.json`, and `journal.json` as required context whenever they exist.

### `rs autopilot orient`

Writes the cold-start orientation packet a fresh AI session should read first. It refreshes the capability atlas, AI playbook, capsule, cockpit, run timeline, command guard, execution runbook, and flight recorder artifacts, then emits `orientation.json` plus `orientation.md` with session mode, read order, operating rules, safe claims, forbidden claims, artifact paths, and the exact next command.

```powershell
rs autopilot orient --run-dir .rs\autopilot\runs\tycoon --format json
rs autopilot orient "add quests and a shop" --root .rs\autopilot\runs
```

Use this when context has been compacted, a new model is taking over, or the agent needs one packet that says what to read, what not to claim, and what to run next. The packet is non-mutating and does not replace approval, live-gate, rehearsal, apply, rollback, or proof evidence.

### `rs autopilot review-pack`

Writes one creator/AI review packet for a run. It refreshes proof, acceptance, approval, privacy, evidence kit, and capsule artifacts, then emits `review-pack.json` plus `review-pack.md` with decision gates, safe-to-say claims, do-not-say claims, blocker/warning summaries, trusted artifact paths, and exact next commands.

```powershell
rs autopilot review-pack .rs\autopilot\runs\tycoon --format json
rs autopilot review-pack .rs\autopilot\runs\tycoon --no-create-evidence-dirs
```

Use this before presenting a generated run to the creator for approval or handing the run to another AI reviewer. The packet is privacy-gated and still refuses to treat offline readiness as live demo proof.

### `rs autopilot publish-review`

Publishes an existing offline Autopilot run summary into the Studio `rs Autopilot` review panel. It reads `plan.json`, optional `preview.json`, `apply.json`, `review-pack.json`, `approval.json`, `proof.json`, optional `companion.json`, optional `best-friend-arc.json`, optional `best-friend.json`, optional `best-friend-pilot.json`, and optional `best-friend-runner.json`, then sends a non-mutating review payload through the bridge and writes `studio-review.json` plus `studio-review.md` as the publish receipt.

```powershell
rs autopilot publish-review .rs\autopilot\runs\tycoon --format json
rs autopilot publish-review .rs\autopilot\runs\tycoon --companion .rs\autopilot\companion.json --studio "Demo Place"
rs autopilot publish-review .rs\autopilot\runs\tycoon --arc .rs\autopilot\runs\tycoon\best-friend-arc.json --format json
rs autopilot publish-review .rs\autopilot\runs\tycoon --best-friend .rs\autopilot\runs\tycoon\best-friend.json --format json
rs autopilot publish-review .rs\autopilot\runs\tycoon --best-friend-pilot .rs\autopilot\runs\tycoon\best-friend-pilot.json --format json
rs autopilot publish-review .rs\autopilot\runs\tycoon --best-friend-runner .rs\autopilot\runs\tycoon\best-friend-runner.json --format json
```

Use this when the AI wants the creator to see the same claim boundaries, blockers, next actions, operation counts, artifact links, selected wow moment, checked post-demo reply, best-friend read order, first safe action, checked opening reply, co-pilot status, selected protected command, checked companion message, bounded runner state, and approval boundary inside Roblox Studio before any live preview or apply. The command updates only the plugin review panel; it does not select a proposal, approve a run, apply changes, execute another best-friend action, or prove live playtest success.

### `rs autopilot proof`

Writes the proof ledger for one run. It maps each important claim to a concrete artifact or missing proof item: plan schema, generated sources, source audit, preview safety, preview integrity, bundle verification, handoff, certification, gameplay verdict, playtest checklist, live playtest result, apply result, rollback proof, and live readiness.

```powershell
rs autopilot proof .rs\autopilot\runs\tycoon --format json
rs autopilot proof .rs\autopilot\runs\tycoon --out .rs\reviews\tycoon-proof.json
```

Use this before telling the creator a run is demo-ready or production-ready. The command does not contact Studio or mutate anything; it turns unsupported success claims into `doNotClaim` entries and exact next commands.

### `rs autopilot acceptance`

Writes the creator-intent acceptance scorecard for one run. It compares the prompt or plan request against expected recipes, generated files, preview/source-audit/bundle/handoff proof, gameplay verdict, playtest checklist, and final live proof. It separates `offlineSatisfied` from `finalSatisfied` so an AI can say "ready for live proof" without claiming the game has already been demo-proven.

```powershell
rs autopilot acceptance .rs\autopilot\runs\tycoon --format json
rs autopilot acceptance .rs\autopilot\runs\tycoon "make a tycoon with shop and saves"
```

Use this before closing a creator request. The command does not mutate Studio; failed or missing criteria become exact next commands instead of vague follow-up prose.

### `rs autopilot fulfillment`

Writes the creator-promise fulfillment checklist for one run. It refreshes proof, acceptance, and trace artifacts, then maps inferred recipes, generated source files, offline review packets, live apply, playtest, rollback, and health evidence into pass/warn/missing checklist items. Missing deterministic recipe promises route to `promise-loop` and `satisfy` next actions instead of generic refresh advice.

```powershell
rs autopilot fulfillment .rs\autopilot\runs\tycoon --format json
rs autopilot fulfillment .rs\autopilot\runs\shop "make a shop with coins" --markdown .rs\reviews\shop-fulfillment.md
```

Use this before telling the creator the request is done. `needsLiveProof` means the offline promise is covered but live apply/playtest/rollback/health proof is still missing; `fulfilled` is the only status that supports a final completion claim.

### `rs autopilot completion-audit`

Writes the prompt-to-artifact completion checklist for one run. It refreshes `closeout.json`, `fulfillment.json`, proof, acceptance, trace, privacy, rollback, and review evidence, then writes `completion-audit.json` plus `completion-audit.md` with the objective, checklist items, safe claims, forbidden claims, blockers, warnings, and exact next actions.

```powershell
rs autopilot completion-audit .rs\autopilot\runs\tycoon --format json
rs autopilot completion-audit .rs\autopilot\runs\shop "make a shop with coins" --markdown .rs\reviews\shop-completion-audit.md
```

Use this as the final AI checkpoint before saying "done" or marking a goal complete. `complete` is the only status that supports a final completion claim. `needsWork` and `needsLiveProof` preserve the first missing artifact-backed requirement and route the model to the next safe command without mutating Studio.

### `rs autopilot deliver`

Writes the creator-facing delivery packet from completion-audit evidence. It refreshes `completion-audit.json`, then writes `delivery.json` plus `delivery.md` with the exact message an AI can send, the completion status, the first missing item when not done, safe claims, forbidden claims, blockers, warnings, artifacts, and next actions.

```powershell
rs autopilot deliver .rs\autopilot\runs\tycoon --format json
rs autopilot deliver .rs\autopilot\runs\shop "make a shop with coins" --markdown .rs\reviews\shop-delivery.md
```

Use this immediately before a human-facing update when the AI wants to avoid improvising from raw artifacts. It is safe even when the request is not done: the message says what remains missing and names the next command. It never mutates Studio, approves apply, publishes, or upgrades offline evidence into live proof.

### `rs autopilot satisfy`

Turns missing creator-promise recipe gaps into a safe offline patch run. It refreshes `fulfillment.json`, selects missing inferred recipes such as `collectibleCoin`, writes `satisfy.json` plus `satisfy.md`, creates a patch run, writes the normal offline review packet, compares source vs. patch, and writes an ordered sequence for review.

```powershell
rs autopilot satisfy .rs\autopilot\runs\shop "make a shop with coins" --format json
rs autopilot satisfy .rs\autopilot\runs\shop --patch-run .rs\autopilot\runs\shop-coins --max-recipes 1
rs autopilot satisfy .rs\autopilot\runs\shop "make a shop with coins" --dry-run --format json
```

Use this when `fulfillment` reports `gapsFound` because a requested deterministic recipe is missing. The command never applies to Studio; it only prepares the patch, comparison, sequence, and exact live-gate/apply next actions.

### `rs autopilot promise-loop`

Runs the offline promise repair loop for a source run and creator prompt. It refreshes source fulfillment, computes expected recipe coverage across the source run plus patch runs, creates one patch run per missing recipe batch, writes comparisons, and emits a final `sequence.json` for the ordered source-plus-patch review.

```powershell
rs autopilot promise-loop .rs\autopilot\runs\shop "make a shop with coins and quests" --format json
rs autopilot promise-loop .rs\autopilot\runs\shop "make a full starter game" --max-steps 4 --max-recipes 2
rs autopilot promise-loop .rs\autopilot\runs\shop "make a shop with coins" --dry-run --out .rs\autopilot\promise-loop-shop
```

Use this when an AI should keep closing deterministic promise gaps without manually repeating `fulfillment`, `satisfy`, `compare`, and `sequence`. The command is still offline-only; live apply, playtest, rollback, and health proof remain separate gates. `next`, `opportunities`, `work-order`, `cycle`, and `act` can route to this command when the selected creator prompt implies deterministic recipes missing from the source run.

### `rs autopilot rollback`

Writes the rollback readiness packet for one applied run. It reads `apply.json`, verifies the named rollback package still exists, computes the safe restore parent for child scopes, and writes `rollback.json` plus `rollback.md` with dry-run and approved restore commands.

```powershell
rs autopilot rollback .rs\autopilot\runs\tycoon --format json
rs autopilot rollback .rs\autopilot\runs\tycoon --markdown .rs\reviews\tycoon-rollback.md
```

Use this before telling the creator a run is undoable. The command is non-mutating; it only recommends `rs package import --dry-run` and approved restore commands when the rollback artifact is present and the scope is safe to restore automatically.

### `rs autopilot approval`

Writes the creator approval packet for one ready run. It reads `plan.json`, `preview.json`, and `certification.json`, then emits `approval.json` plus `approval.md` with the exact approval prompt, live-readiness command, mutating apply command, risk level, changed paths, generated files, blockers, warnings, and claims the AI must not make yet.

```powershell
rs autopilot approval .rs\autopilot\runs\tycoon --format json
rs autopilot approval .rs\autopilot\runs\tycoon --markdown .rs\reviews\tycoon-approval.md
```

Use this immediately before asking the creator for live Studio mutation approval. It does not contact Studio or apply changes; the packet is blocked unless certification says the run is ready to apply.

### `rs autopilot privacy`

Scans a run folder for unredacted secret-like artifact values. It writes `privacy.json` plus `privacy.md` with scanned file counts, findings, blockers, claim-ready text, do-not-claim text, and refresh actions. Structured JSON is inspected by key so fields such as `apiKey`, `token`, `secret`, `password`, and `credential` must be redacted or empty.

```powershell
rs autopilot privacy .rs\autopilot\runs\tycoon --format json
rs autopilot privacy .rs\autopilot\runs\tycoon --markdown .rs\reviews\tycoon-privacy.md
```

Use this before handing run artifacts to another AI model, CI job, or reviewer. The command is non-mutating and redacts evidence in its own report.

### `rs autopilot next`

Chooses the next best Autopilot move for an AI agent. It can inspect a specific run or the project run root, then writes `next.json` plus `next.md` with one recommended action, alternatives, blockers, warnings, and evidence from decisions, alignment, creator-promise recipe coverage, bundle, certification, gameplay critique, and project memory artifacts.

```powershell
rs autopilot next --root .rs\autopilot\runs --format json
rs autopilot next --run-dir .rs\autopilot\runs\shop
rs autopilot next "add quests and a shop" --root .rs\autopilot\runs --markdown .rs\autopilot\next.md
```

Use this when an AI is resuming a session and should not guess whether to align with creator decisions, satisfy missing promises, critique, improve, certify, hand off, rehearse live proof, or start a new kickoff. If `decisions.json` exists without `alignment.json`, `next` recommends `rs autopilot align`; if alignment is blocked, `next` blocks continuation until the decision drift is resolved. Missing deterministic creator promises become `promise-loop` recommendations before generic critique, certification, or live-readiness work. Certified ready-to-apply runs route to `rehearsal` before raw readiness/apply commands. The command does not contact Studio or mutate anything.

### `rs autopilot roadmap`

Writes a multi-step execution roadmap for AI agents. It reads project memory, the active run, `next`, gameplay critique, certification, bundle state, alignment state, and an optional creator prompt, then emits `roadmap.json` plus `roadmap.md` with milestones, ranked backlog items, exact commands, expected artifacts, blockers, warnings, and refresh actions.

```powershell
rs autopilot roadmap --root .rs\autopilot\runs --format json
rs autopilot roadmap "add quests and a shop" --root .rs\autopilot\runs --out .rs\autopilot\roadmap.json
```

Use this when the agent needs a short execution backlog rather than only one next command. The roadmap stays offline and non-mutating; live work remains gated behind `ready`, explicit apply approval, rollback, and validation.

If `next` reports blocked decision drift, roadmap status is also `blocked` and the first backlog item is the alignment repair command.

### `rs autopilot opportunities`

Ranks evidence-backed next opportunities for an AI agent. It reads `memory`, `next`, `roadmap`, the selected run artifacts, creator decisions, alignment, creator-promise coverage, and journal state, then emits `opportunities.json` plus `opportunities.md` with scores, confidence, reasons, commands, expected artifacts, blockers, safe claims, and forbidden claims.

Use this when the agent needs strategic judgment, not only the single `next` command. `opportunities` stays offline and non-mutating. It never treats a high score as proof that work is implemented, and it never bypasses decision alignment, privacy, approval, claim-check, or live-gate blockers.

```powershell
rs autopilot opportunities --run-dir .rs\autopilot\runs\starter-game --format json
rs autopilot opportunities "add quests and onboarding" --root .rs\autopilot\runs --markdown .rs\autopilot\opportunities.md
```

### `rs autopilot work-order`

Turns one ranked opportunity into an exact AI execution packet. It writes `work-order.json` plus `work-order.md`, refreshes `opportunities.json`, selects the top opportunity by default or a named `--opportunity`, and records the objective, execution steps, validation commands, expected artifacts, stop conditions, safe claims, and forbidden claims.

Use this when the AI is ready to act on an opportunity and needs a precise work order instead of a dashboard. The command stays offline and non-mutating. It exits non-zero if the selected opportunity is missing, blocked, lacks an executable command, or would mutate Studio without the approval/live-gate path.

```powershell
rs autopilot work-order --run-dir .rs\autopilot\runs\starter-game --format json
rs autopilot work-order --run-dir .rs\autopilot\runs\starter-game --opportunity "Record AI work journal" --markdown .rs\reviews\starter-work-order.md
```

### `rs autopilot work-check`

Checks whether a selected work order has actually produced the expected evidence. It reads `work-order.json`, verifies the selected command exists, checks each file-like expected artifact under the run directory, verifies `bundle.json` when present, inspects `journal.json` for command continuity, and writes `work-check.json` plus `work-check.md`.

Use this after executing a work order and before reporting progress. The command stays offline and non-mutating. It exits non-zero when the work order is missing, blocked, missing expected artifacts, or depends on manual evidence that cannot be verified as a file.

```powershell
rs autopilot work-check .rs\autopilot\runs\starter-game --format json
rs autopilot work-check .rs\autopilot\runs\starter-game --work-order .rs\reviews\starter-work-order.json --markdown .rs\reviews\starter-work-check.md
```

### `rs autopilot cycle`

Runs one offline AI work cycle for a run folder. It refreshes `opportunities.json`, creates `work-order.json` if missing, refreshes the bundle before `work-check`, writes `work-check.json`, claim-checks the work-check safe claim when evidence is present, composes `response.json` when the claim is supported, and writes `cycle.json` plus `cycle.md` with the next exact action.

Use this when an AI needs one command that answers "what do I do now?" without mutating Studio or executing arbitrary shell commands. The cycle stops at `executeWorkOrder`, `recordManualEvidence`, `rewriteClaim`, or `readyToReport` and preserves the exact command queue for the next turn.

```powershell
rs autopilot cycle .rs\autopilot\runs\starter-game --format json
rs autopilot cycle .rs\autopilot\runs\starter-game --prompt "add quests and onboarding" --markdown .rs\reviews\starter-cycle.md
```

### `rs autopilot diagnose`

Diagnoses why an offline AI cycle or command is stuck. It reads `cycle.json`, `work-check.json`, `claim-check.json`, `journal.json`, bundle verification, recorded playtest results, alignment, and privacy status when present, then writes `diagnosis.json` plus `diagnosis.md`.

Use this after a failed command, a blocked cycle, missing work-order evidence, stale bundle state, or unsupported claim wording. The command is non-mutating: it classifies incidents and gives exact recovery commands, but it never claims repair, live Studio success, playtest success, rollback readiness, or publish readiness.

```powershell
rs autopilot diagnose .rs\autopilot\runs\starter-game --format json
rs autopilot diagnose .rs\autopilot\runs\starter-game --command "rs autopilot cycle .rs\autopilot\runs\starter-game --format json" --result "failed" --error "selected command has not been run yet"
```

### `rs autopilot command-guard`

Validates an AI-proposed Autopilot command sequence before execution. It accepts repeated `--command` values, a `--from-file` list, or discovers a run's existing command queue from orientation, cockpit, agenda, diagnosis, cycle, or work-order artifacts. It writes `command-guard.json` plus `command-guard.md` with command kinds, known/unknown status, `act` support, expected artifacts, live-readiness flags, mutation flags, blockers, warnings, safe-to-say claims, do-not-say claims, and next actions.

```powershell
rs autopilot command-guard --run-dir .rs\autopilot\runs\starter-game --format json
rs autopilot command-guard --command "rs autopilot proof .rs\autopilot\runs\starter-game --format json" --command "rs autopilot apply --plan .rs\autopilot\runs\starter-game\plan.json --yes"
```

Use this before `act`, manual CLI execution, or any copied command queue. The guard never executes commands. Unsupported command names are blockers; live or mutating commands route to setup, rehearsal, approval, ready/live-gate, rollback, and proof gates before execution.

### `rs autopilot self-check`

Checks proposed AI output before the model speaks or acts. It combines `claim-check` and `command-guard` into `self-check.json` plus `self-check.md`, with optional child `self-check-claim-check.json` and `self-check-command-guard.json` evidence. It accepts repeated `--claim`, `--message`, `--command`, and `--from-file` values.

```powershell
rs autopilot self-check .rs\autopilot\runs\starter-game --claim "The offline run is ready for review" --command "rs autopilot proof .rs\autopilot\runs\starter-game --format json" --format json
rs autopilot self-check .rs\autopilot\runs\starter-game --message "I can show the offline candidate, but it has not been applied to Studio." --format json
```

Use this as the AI's last preflight before creator-facing wording or command execution. `readyToProceed` means the claims are supported or absent and the proposed commands are safe offline. `blocked` or `needsRewriteOrGate` means the AI must rewrite the message, stop at approval/readiness, or run the recommended safer command. It is non-mutating and does not execute the command queue.

### `rs autopilot runbook`

Writes an execution runbook from the same guarded command inputs used by `command-guard`. It writes `runbook.json` plus `runbook.md`, refreshes a companion `command-guard.json` / `.md`, then splits the reviewed queue into a safe offline prefix and a gated suffix. Each step records the source command, exact executable command, command kind, `act` support, expected artifacts, done-when checks, stop-if checks, live-readiness flags, mutation flags, blockers, warnings, and next actions.

```powershell
rs autopilot runbook --run-dir .rs\autopilot\runs\starter-game --format json
rs autopilot runbook "make a cozy shop with quests" --format json
rs autopilot runbook --command "rs autopilot proof .rs\autopilot\runs\starter-game --format json" --command "rs autopilot apply --plan .rs\autopilot\runs\starter-game\plan.json --yes"
```

Use this when an AI should know exactly what it may do next without executing the whole queue. The runbook never runs commands. It recommends only one safe prefix step at a time, requires rerunning the runbook after that step, and stops before any live Studio, setup, ready, apply, publish, or mutation boundary until rehearsal, approval, rollback, ready/live-gate, and proof artifacts are current.

### `rs autopilot flight-recorder`

Writes a run-level black box for AI continuation. It reads existing run artifacts such as `command-guard.json`, `runbook.json`, `act.json`, `sprint.json`, `loop.json`, `journal.json`, `proof.json`, `acceptance.json`, `rehearsal.json`, `approval.json`, `live-gate.json`, `apply.json`, `rollback.json`, and `closeout.json`, then writes `flight-recorder.json` plus `flight-recorder.md`.

```powershell
rs autopilot flight-recorder .rs\autopilot\runs\starter-game --format json
rs autopilot flight-recorder .rs\autopilot\runs\starter-game --markdown .rs\reviews\starter-flight.md
```

Use this after long AI sessions, compaction, sprint/loop work, or before a handoff. The recorder never executes commands. It extracts recorded commands, classifies them through command-guard rules, summarizes gates and blockers, counts safe and forbidden claim evidence, and gives the next safe recovery or proof command without treating recorded plans as completed work.

### `rs autopilot navigator`

Writes a concise AI operating card. It refreshes orientation first, which in turn refreshes command guard, runbook, and flight recorder for the selected run, then writes `navigator.json` plus `navigator.md` with the required read order, situation signals, first safe action, next command, stop conditions, safe-to-say claims, forbidden claims, blockers, warnings, and artifact paths.

```powershell
rs autopilot navigator --run-dir .rs\autopilot\runs\starter-game --format json
rs autopilot navigator "make a cozy shop with quests" --root .rs\autopilot\runs
```

Use this as the first command for a fresh AI that needs to operate, not just inspect. The navigator never executes work. It prefers one runbook safe-prefix action when available, warns about noisy historical commands from the recorder, and requires a refresh after one action before the model continues or reports progress.

### `rs autopilot model-pack`

Writes a compact model-ready context pack for a run. It refreshes `navigator.json` / `.md` and `delivery.json` / `.md`, then writes `model-pack.json` plus `model-pack.md` with a resume prompt, required read order, redacted bounded snippets from key artifacts, source artifact links, safe-to-say claims, do-not-say guardrails, blockers, warnings, and next actions.

```powershell
rs autopilot model-pack .rs\autopilot\runs\starter-game --format json
rs autopilot model-pack .rs\autopilot\runs\starter-game "make a cozy shop" --max-chars 18000 --format json
```

Use this when context has been compacted, a new model is taking over, or the AI needs one packet that is small enough to load while still pointing back to authoritative source artifacts. The pack is non-mutating. Snippets are resume context, not proof that commands ran or that completion claims are true.

### `rs autopilot task-pack`

Writes a copy/paste-ready task packet for the next AI agent. It refreshes `model-pack.json`, `opportunities.json`, and `work-order.json`, then writes `task-pack.json` plus `task-pack.md` with one task prompt, primary command, allowed command list, validation commands, expected artifacts, acceptance checks, stop conditions, safe-to-say claims, do-not-say guardrails, blockers, warnings, and source artifact links.

```powershell
rs autopilot task-pack .rs\autopilot\runs\starter-game --format json
rs autopilot task-pack .rs\autopilot\runs\starter-game "make a cozy shop" --opportunity "Record AI work journal" --max-chars 18000 --format json
```

Use this when a fresh coding agent needs to execute exactly one safe Autopilot task instead of reinterpreting the whole run. It is non-mutating and does not execute the task. Live-required tasks are marked as gated, mutating tasks are blocked, and source artifacts remain authoritative over embedded instructions.

### `rs autopilot best-friend`

Writes one fresh-AI launch packet for a Roblox build companion. It refreshes `remember.json`, `model-pack.json`, `task-pack.json`, `opportunities.json`, and `work-order.json`, then writes `best-friend.json` plus `best-friend.md` with an opening prompt, checked opening reply, companion contract, context cards, required read order, first safe action, protected `act` command when supported, safe claims, forbidden claims, blockers, warnings, and source artifact links. It also writes `best-friend-self-check.json` plus optional Markdown, using `claim-check` and `command-guard` to preflight the first safe action and launch claim before a fresh model acts.

```powershell
rs autopilot best-friend .rs\autopilot\runs\starter-game --root .rs\autopilot\runs --format json
rs autopilot best-friend .rs\autopilot\runs\starter-game "make a cozy shop" --opportunity "Record AI work journal" --max-chars 18000 --format json
```

Use this when a new or context-compacted AI should start from the strongest available project context instead of manually stitching together memory, orientation, model-pack, task-pack, self-check, protected execution, and creator-facing wording. It is non-mutating: it does not execute the task, approve apply, mutate Studio, prove playtests, publish, or mark work complete.

### `rs autopilot best-friend-check`

Audits whether a fresh AI can safely start from the best-friend launch packet. It refreshes `best-friend.json`, then writes `best-friend-check.json` plus `best-friend-check.md` with a launch-control prompt, checklist, checked opening reply, protected first action, required read order, Studio review visibility, safe claims, forbidden claims, blockers, warnings, and exact repair commands.

```powershell
rs autopilot best-friend-check .rs\autopilot\runs\starter-game --root .rs\autopilot\runs --format json
rs autopilot best-friend-check .rs\autopilot\runs\starter-game "make a cozy shop" --opportunity "Record AI work journal" --format json
```

Use this when a new model is about to act or speak and needs one final launch-control receipt. The command is non-mutating: it does not execute the protected first action, apply to Studio, publish, upload, prove playtests, or mark work complete. Missing Studio review visibility is a warning with a `publish-review` repair action, not proof that the packet is unusable.

### `rs autopilot best-friend-rescue`

Recovers a blocked or confused AI companion session without guessing from chat history. It refreshes `best-friend-check.json`, optionally diagnoses a failed command/result/error into `best-friend-rescue-diagnosis.json`, writes `best-friend-rescue.json` plus `best-friend-rescue.md`, selects the safest repair action, and self-checks the recovery message as `best-friend-rescue-self-check.json`.

```powershell
rs autopilot best-friend-rescue .rs\autopilot\runs\starter-game --root .rs\autopilot\runs --format json
rs autopilot best-friend-rescue .rs\autopilot\runs\starter-game --command "rs autopilot act .rs\autopilot\runs\starter-game --format json" --result "failed" --error "missing evidence" --format json
```

Use this when the model hit a blocker, a command failed, launch control is stale, or the AI needs a checked recovery message before speaking. The command does not execute the selected repair, mutate Studio, publish, upload, prove playtests, or mark work complete; it only prepares the next recovery route and honest wording.

### `rs autopilot best-friend-mentor`

Writes a read-first coaching packet for a fresh or resumed AI companion. It refreshes `best-friend-check.json`, then writes `best-friend-mentor.json` plus `best-friend-mentor.md` with a mentor prompt, current focus, required read order, coaching cards, mistake traps, safe claims, forbidden claims, blockers, warnings, artifacts, and the next protected action or rescue command.

```powershell
rs autopilot best-friend-mentor .rs\autopilot\runs\starter-game --root .rs\autopilot\runs --format json
rs autopilot best-friend-mentor .rs\autopilot\runs\starter-game "coach the next AI before it acts" --opportunity "Record AI work journal" --format json
```

Use this when an AI needs judgment, not just a command: what to read, why the chosen action matters, which overclaims or unsafe shortcuts to avoid, and when to run `best-friend-rescue` before speaking or acting. The command is non-mutating and does not execute the protected action, mutate Studio, publish, upload, prove playtests, or mark work complete.

### `rs autopilot best-friend-pilot`

Runs the self-healing one-move companion loop. It first refreshes `best-friend-mentor.json`; when launch control is ready, it writes `first-turn.json`, executes one protected offline action through the internal `act` dispatcher, refreshes `best-friend-reply.json`, and returns `companionMessage` only when the reply self-check is ready. When launch control is blocked, it writes `best-friend-rescue.json` instead of executing work.

```powershell
rs autopilot best-friend-pilot .rs\autopilot\runs\starter-game --root .rs\autopilot\runs --format json
rs autopilot best-friend-pilot .rs\autopilot\runs\starter-game "take one safe companion step" --opportunity "Record AI work journal" --dry-run --format json
```

Use this when a model should behave like a co-pilot rather than a command copier: coach itself, take exactly one safe offline move if launch control allows it, prepare checked wording, then stop. The command never crosses live Studio, apply, publish, upload, rollback, or playtest boundaries, and it does not mark the creator request complete.

### `rs autopilot best-friend-control`

Writes the operator receipt for a resumed AI companion. It refreshes `best-friend-check.json`, reads the latest `best-friend-pilot.json` and `studio-review.json` when present, then decides whether the AI should run the pilot, recover with rescue, publish the co-pilot receipt into Studio, refresh the checked reply, or send the checked companion message exactly.

```powershell
rs autopilot best-friend-control .rs\autopilot\runs\starter-game --root .rs\autopilot\runs --format json
rs autopilot best-friend-control .rs\autopilot\runs\starter-game "what should I do next?" --opportunity "Record AI work journal" --format json
```

Use this as the first command after context compaction, a pilot run, or Studio review publishing. It is non-mutating and does not execute the selected branch; it writes `best-friend-control.json` plus `best-friend-control.md` with the selected branch, exact next command, operator steps, safe claims, forbidden claims, and checked companion message when it is ready to send.

### `rs autopilot best-friend-operate`

Executes exactly one offline branch selected by `best-friend-control`, then refreshes control and stops. It can run `best-friend-pilot`, `best-friend-rescue`, or `best-friend-reply` when that branch is selected. If control says the next branch is Studio publishing, it stops with the exact `publish-review` command; if control says the message is ready, it returns the checked `companionMessage` without executing anything.

```powershell
rs autopilot best-friend-operate .rs\autopilot\runs\starter-game --root .rs\autopilot\runs --format json
rs autopilot best-friend-operate .rs\autopilot\runs\starter-game "take the next safe operator step" --dry-run --format json
```

Use this when a model should advance one safe companion step without manually copying a command from `best-friend-control`. It writes `best-friend-operate.json`, `best-friend-operate.md`, `best-friend-operate-control-before.json`, and, when an offline branch executes, `best-friend-operate-control-after.json`. It never publishes to Studio, applies, uploads, proves playtests, claims completion, or executes a second branch.

### `rs autopilot best-friend-runner`

Runs bounded `best-friend-operate` steps until a real operator boundary stops the AI. It writes `best-friend-runner.json` plus `best-friend-runner.md`, preserves per-step receipts such as `best-friend-runner-step-01.json`, and stops at checked speech, Studio publish, blockers, repeated branch risk, dry-run, or `--max-steps`.

```powershell
rs autopilot best-friend-runner .rs\autopilot\runs\starter-game --root .rs\autopilot\runs --max-steps 3 --format json
rs autopilot best-friend-runner .rs\autopilot\runs\starter-game "keep operating safely" --dry-run --format json
```

Use this when a model should keep making safe offline progress without manually rerunning `best-friend-operate`. It never publishes to Studio, applies, uploads, proves playtests, claims completion, or repeats an already executed operator branch without a fresh control/rescue review.

### `rs autopilot first-turn`

Executes exactly one protected first action from the best-friend launch packet, writes `first-turn.json` plus `first-turn.md`, records the nested `act` receipt as `first-turn-act.json`, then refreshes `best-friend.json` for the next AI turn. It first writes `first-turn-best-friend-before.json` so the chosen action, launch preflight, and opening reply are preserved before execution.

```powershell
rs autopilot first-turn .rs\autopilot\runs\starter-game --root .rs\autopilot\runs --format json
rs autopilot first-turn .rs\autopilot\runs\starter-game "make a cozy shop" --opportunity "Record AI work journal" --dry-run --format json
```

Use this when a fresh AI should make one safe offline move without improvising a command. It only runs commands accepted by the internal `act` dispatcher, stops on best-friend blockers or live/mutating gates, and does not approve apply, mutate Studio, prove playtests, publish, or mark the creator request complete.

### `rs autopilot best-friend-loop`

Runs bounded protected best-friend turns. It previews the next `best-friend` packet, refuses repeated commands before execution, calls `first-turn` for each non-repeated protected action, writes per-turn receipts such as `best-friend-loop-turn-01.json` plus `best-friend-loop-turn-01-act.json`, and emits `best-friend-loop.json` plus `best-friend-loop.md`.

```powershell
rs autopilot best-friend-loop .rs\autopilot\runs\starter-game --root .rs\autopilot\runs --max-steps 3 --format json
rs autopilot best-friend-loop .rs\autopilot\runs\starter-game --opportunity "Record AI work journal" --dry-run --format json
```

Use this when an AI should make a short safe offline run without manually alternating between `best-friend`, `first-turn`, and `self-check`. The loop never shells out to arbitrary commands, never repeats the same protected action, stops at blockers or live/mutating gates, and does not approve apply, mutate Studio, prove playtests, publish, or claim the request is complete.

### `rs autopilot best-friend-reply`

Drafts the creator-facing update after best-friend work and checks it before the AI speaks. It reads `best-friend-loop.json`, `first-turn.json`, or `best-friend.json`, writes `best-friend-reply.json` plus `best-friend-reply.md`, and runs the message through `self-check` as `best-friend-reply-self-check.json`.

```powershell
rs autopilot best-friend-reply .rs\autopilot\runs\starter-game --format json
rs autopilot best-friend-reply .rs\autopilot\runs\starter-game --markdown .rs\reviews\best-friend-reply.md
```

Use this before sending the creator a status update after `best-friend`, `first-turn`, or `best-friend-loop`. The reply packet can say what the receipts prove, but it must not claim Studio apply, publishing, uploading, playtesting, or completion unless separate evidence supports those claims.

### `rs autopilot best-friend-turn`

Handles one AI companion operating turn. It can route the latest creator message through `chat`, run bounded protected best-friend work through `best-friend-loop`, draft the checked update through `best-friend-reply`, and write `best-friend-turn.json` plus `best-friend-turn.md` as the top-level receipt.

```powershell
rs autopilot best-friend-turn .rs\autopilot\runs\starter-game --message "make the shop button brighter" --format json
rs autopilot best-friend-turn .rs\autopilot\runs\starter-game --opportunity "Record AI work journal" --max-steps 1 --format json
```

Use this when a fresh or context-compacted AI needs one disciplined cycle from creator message to protected offline action to checked reply. The command never treats chat wording as live apply permission, never shells arbitrary text, stops at approval/live gates, and does not claim Studio mutation, publishing, uploading, playtesting, or completion.

### `rs autopilot best-friend-session`

Bootstraps or resumes an AI best-friend session from first contact. Without an existing run, it uses `session` to scout/start the offline run, then runs `best-friend-turn`; with `--run-dir`, it resumes that run directly. It writes `best-friend-session.json` plus `best-friend-session.md` with the bootstrap status, selected run, protected turn status, checked message, safe claims, forbidden claims, blockers, and next actions.

```powershell
rs autopilot best-friend-session "make a tycoon with shop and saves" --assume --format json
rs autopilot best-friend-session --run-dir .rs\autopilot\runs\starter-game --message "make the shop button brighter" --max-steps 1 --format json
```

Use this as the top-level command when a model is meeting the creator cold or resuming a known run and wants one safe companion cycle. It never treats chat as approval, never crosses live apply/publish/upload boundaries, and only returns creator-facing wording after the nested best-friend reply self-check passes.

### `rs autopilot wow-session`

Bootstraps or resumes an AI best-friend session, then prepares the full offline wow demo loop. It writes `wow-session.json` plus `wow-session.md`, refreshes `best-friend-session.json`, builds or plans the selected wow candidate through `creator-demo`, and records the selected idea, recommended run, demo title, creator message, safe claims, forbidden claims, blockers, warnings, and next actions.

```powershell
rs autopilot wow-session "make a tycoon with shop and saves" --assume --format json
rs autopilot wow-session --run-dir .rs\autopilot\runs\starter-game --idea "First Upgrade Ceremony" --format json
```

Use this when the model should move from first contact or resumed context to a reviewable creator demo without manually stitching together `best-friend-session`, `wow-plan`, `moment-sprint`, `moment-decision`, and `creator-demo`. It is still offline and proof-gated: it does not apply to Studio, publish, upload, live playtest, record approval, or prove production readiness.

### `rs autopilot best-friend-arc`

Runs the top-level companion journey as one artifact-backed receipt. It writes `best-friend-arc.json` plus `best-friend-arc.md`, refreshes `wow-session.json`, and, when `--message` is supplied, routes the post-demo reaction through `demo-session` so the AI gets a checked reply and refreshed memory without stitching commands together from chat.

```powershell
rs autopilot best-friend-arc "make a tycoon with shop and saves" --assume --format json
rs autopilot best-friend-arc --run-dir .rs\autopilot\runs\starter-game --message "Looks good but make the shop button brighter" --format json
```

Use this when a model should behave like the creator's continuous Roblox build companion: prepare the proof-bound wow demo, then optionally handle the creator reaction, learn from it, and leave the next AI with one read-first receipt. It remains non-mutating and never treats a demo, feedback, approval-like wording, or memory update as live Studio apply, upload, publish, playtest, rollback readiness, or production proof.

### `rs autopilot squad-pack`

Writes a multi-agent assignment board for a run. It refreshes `model-pack.json` and `opportunities.json`, then writes `squad-pack.json` plus `squad-pack.md` with a coordination prompt, per-agent task prompts, role labels, ownership boundaries, primary commands, allowed commands, validation commands, expected artifacts, stop conditions, safe-to-say claims, do-not-say guardrails, blockers, warnings, and source links.

```powershell
rs autopilot squad-pack .rs\autopilot\runs\starter-game --format json
rs autopilot squad-pack .rs\autopilot\runs\starter-game "make a cozy shop" --max-tasks 3 --max-chars 18000 --format json
```

Use this when several AI agents can work from the same run without trampling each other. The pack is non-mutating and does not execute assignments. Each assignment must stay inside its ownership notes, stop at live or mutating gates, and refresh task/squad evidence before reporting progress.

### `rs autopilot squad-review`

Reviews a squad assignment board after parallel agent work. It refreshes `squad-pack.json`, checks each assignment's expected artifacts, checks whether `journal.json` records the assignment command, detects duplicated expected-artifact ownership, and writes `squad-review.json` plus `squad-review.md` with an integration prompt, per-assignment review status, conflicts, safe-to-say claims, do-not-say guardrails, blockers, warnings, and next actions.

```powershell
rs autopilot squad-review .rs\autopilot\runs\starter-game --format json
rs autopilot squad-review .rs\autopilot\runs\starter-game --max-tasks 3 --format json
```

Use this when multiple agents have worked from a `squad-pack` and the lead AI needs to know what is actually ready to integrate. The review is non-mutating; it does not merge work, apply to Studio, or authorize completion claims. A creator-facing summary still needs claim-check evidence.

### `rs autopilot wow-plan`

Ranks safe wow-factor candidates for an existing run. It reads the prompt plus recognized artifacts such as storyboard, gameplay critique, simulation, feature graph, world blueprint, showcase, onboarding, style guide, persistence, social, monetization, liveops, and asset brief, then writes `wow-plan.json` plus `wow-plan.md` with current signals, ranked ideas, one selected player moment, safe next commands, proof needs, safe-to-say claims, do-not-say guardrails, blockers, warnings, and next actions.

```powershell
rs autopilot wow-plan .rs\autopilot\runs\starter-game --format json
rs autopilot wow-plan .rs\autopilot\runs\starter-game "make the shop reveal memorable" --max-ideas 4 --format json
```

Use this when a run is technically coherent but needs a real product hook before another agent keeps building. The plan is non-mutating; it does not build the feature, apply to Studio, playtest, publish, or prove the wow moment. Follow the selected safe command and then refresh proof, showcase, completion-audit, and claim-check before reporting the idea as implemented.

### `rs autopilot moment-pack`

Turns the selected `wow-plan` idea into an agent-ready implementation packet. It refreshes `wow-plan.json`, selects either the requested `--idea` or the current selected idea, and writes `moment-pack.json` plus `moment-pack.md` with a candidate run path, build lanes, exact safe commands, expected artifacts, proof checklist, validation commands, task prompt, safe-to-say claims, do-not-say guardrails, blockers, warnings, and next actions.

```powershell
rs autopilot moment-pack .rs\autopilot\runs\starter-game --format json
rs autopilot moment-pack .rs\autopilot\runs\starter-game --idea "First Upgrade Ceremony" --max-chars 18000 --format json
```

Use this when the AI has chosen the product hook and a fresh implementation agent needs executable work, not more brainstorming. The packet is non-mutating: it does not run the candidate compose command, apply to Studio, playtest, publish, or prove the moment exists. The first lane creates a separate offline candidate run; later lanes refresh showcase/critique/completion evidence and claim-check the exact summary.

### `rs autopilot moment-sprint`

Executes the selected wow moment as a safe offline candidate sprint. It refreshes `moment-pack.json`, creates a separate candidate run from the selected idea, then refreshes candidate showcase, gameplay critique, proof, acceptance, completion-audit, and claim-check artifacts. It writes `moment-sprint.json` plus `moment-sprint.md` with every step, artifact, warning, blocker, safe-to-say claim, forbidden claim, and next action.

```powershell
rs autopilot moment-sprint .rs\autopilot\runs\starter-game --format json
rs autopilot moment-sprint .rs\autopilot\runs\starter-game --idea "First Upgrade Ceremony" --dry-run --format json
```

Use this when the AI should move from a moment brief to a reviewable offline candidate without copying shell text. The sprint writes files only under the source and candidate run folders. It never applies to Studio, publishes, uploads, runs a live playtest, or proves production readiness; use `ready`, `live-gate`, and approved `apply` later if the creator chooses the candidate.

### `rs autopilot moment-decision`

Compares the reviewed wow candidate against the source run and writes a proof-bound continuation recommendation. It refreshes `moment-sprint.json`, writes candidate `comparison.json`, refreshes the recommended run's `review-pack.json` and `claim-check.json`, then writes `moment-decision.json` plus `moment-decision.md` with evidence, safe claims, forbidden claims, blockers, warnings, and next actions.

```powershell
rs autopilot moment-decision .rs\autopilot\runs\starter-game --format json
rs autopilot moment-decision .rs\autopilot\runs\starter-game --idea "First Upgrade Ceremony" --dry-run --format json
```

Use this after `moment-sprint` when the AI needs to know whether the wow candidate is actually the best continuation to show the creator. The decision is non-mutating: it does not record creator approval, apply to Studio, publish, live playtest, or prove production readiness.

### `rs autopilot creator-demo`

Builds one proof-bound creator presentation packet for the recommended wow run. It refreshes `moment-decision.json`, then refreshes the recommended run's `showcase.json`, `review-pack.json`, `delivery.json`, and `rehearsal.json`. It writes `creator-demo.json` plus `creator-demo.md` with a talk track, proof table, approval boundary, safe claims, forbidden claims, blockers, warnings, and next actions.

```powershell
rs autopilot creator-demo .rs\autopilot\runs\starter-game --format json
rs autopilot creator-demo .rs\autopilot\runs\starter-game --idea "First Upgrade Ceremony" --dry-run --format json
```

Use this when the AI is ready to show the creator the recommended offline wow continuation. The demo packet is non-mutating: it does not publish to Studio, record approval, apply, live playtest, or prove production readiness.

### `rs autopilot demo-response`

Routes the creator's post-demo reaction into the next safe artifact path. It refreshes `creator-demo.json`, classifies the exact response as feedback, approval-like wording, redirection, or checked-response handling, then writes `demo-response.json` plus `demo-response.md`.

```powershell
rs autopilot demo-response .rs\autopilot\runs\starter-game --message "Looks good but make the shop button brighter" --format json
rs autopilot demo-response .rs\autopilot\runs\starter-game --message "Looks good, go ahead" --format json
```

Feedback routes write `feedback.json`, `feedback-patch.json`, and a feedback planner pack for the recommended run. Approval-like routes write `approval.json` and point to `ready` / `live-gate --approved`, but they never treat chat wording as live Studio apply permission. Redirections write `decisions.json`; question-like responses go through checked claims.

### `rs autopilot demo-loop`

Packages a routed post-demo response into the next AI handoff. It refreshes `demo-response.json`, then writes `demo-loop.json` plus `demo-loop.md` with a handoff prompt, command queue, expected artifacts, stop conditions, safe claims, forbidden claims, blockers, warnings, and the embedded route report.

```powershell
rs autopilot demo-loop .rs\autopilot\runs\starter-game --message "Looks good but make the shop button brighter" --format json
rs autopilot demo-loop .rs\autopilot\runs\starter-game --message "Looks good, go ahead" --format json
```

Use this when a fresh AI needs to continue after the creator reacts to the demo. Feedback routes include the feedback-specific planner prompt and adoption/validation commands; approval routes include approval/readiness/live-gate commands and stop before apply; redirection routes point back to wow planning; checked-response routes point to `respond`.

### `rs autopilot demo-session`

Handles one post-demo creator reaction end-to-end. It refreshes `demo-loop.json`, audits follow-through through `demo-check`, drafts the checked wording through `demo-reply`, distills reusable signals through `demo-learn`, consolidates memory through `remember`, and writes `demo-session.json` plus `demo-session.md`.

```powershell
rs autopilot demo-session .rs\autopilot\runs\starter-game --message "Looks good but make the shop button brighter" --format json
rs autopilot demo-session .rs\autopilot\runs\starter-game --message "Looks good, go ahead" --format json
```

Use this when an AI has shown a `creator-demo` packet and now needs to safely respond, preserve learning, and hand off the next route without trusting chat memory. It can return `replyMessage` only after the reply is checked. It never treats feedback as implemented, approval-like wording as apply permission, or memory learning as live Studio mutation, playtest, publish, rollback, or production proof.

### `rs autopilot demo-check`

Audits whether the post-demo follow-up is actually ready to report. It reads `demo-loop.json`, `demo-response.json`, and route-specific artifacts, then writes `demo-check.json` plus `demo-check.md` with check items, safe claims, forbidden claims, blockers, warnings, and next commands.

```powershell
rs autopilot demo-check .rs\autopilot\runs\starter-game --format json
```

Use this before telling the creator that feedback was handled, approval is prepared, direction was recorded, or a checked response is ready. Feedback routes stay `needsFollowup` until an adopted patch run exists and is ready for certification; approval routes can be `approvalPrepared` without crossing live apply; redirection and response routes require their own evidence. `demo-check` is non-mutating and never proves live Studio apply, playtest, publish, or production readiness.

### `rs autopilot demo-reply`

Composes the creator-facing post-demo update from checked route evidence. It refreshes `demo-check.json`, writes `demo-reply-claim-check.json`, then writes `demo-reply.json` plus `demo-reply.md` with the exact message, checked claims, safe claims, forbidden claims, blockers, warnings, and next actions.

```powershell
rs autopilot demo-reply .rs\autopilot\runs\starter-game --format json
```

Use this after `demo-check` when the AI needs wording it can safely send. Feedback routes explicitly say the patch is not implemented until an adopted patch run is reviewed; approval routes say nothing was applied to Studio; redirection routes preserve the new direction without treating it as approval; checked-response routes use claim-check-supported wording. `demo-reply` exits non-zero unless the message is `readyToSend`.

### `rs autopilot demo-learn`

Distills the checked post-demo conversation into reusable planning signals. It refreshes `demo-reply.json`, then writes `demo-learn.json` plus `demo-learn.md` with creator signals, learned preferences, constraints, follow-up themes, recommended memory actions, safe claims, forbidden claims, and next commands.

```powershell
rs autopilot demo-learn .rs\autopilot\runs\starter-game --format json
```

Use this after `demo-reply` when a future AI should inherit what the creator liked, rejected, asked to change, or needed clarified. The artifact recommends refreshing `preferences`, `memory`, and `playbook`, but it does not update those durable project files by itself and does not prove feedback implementation, approval, live apply, playtest, publish, or production readiness.

### `rs autopilot remember`

Consolidates post-demo learning into the durable project context a future AI should read. It refreshes `demo-learn.json`, then writes `remember.json` plus `remember.md` and regenerates `project-memory.json`, `creator-preferences.json`, `game-bible.json`, and `ai-playbook.json` under the project context directory. Its first normal next action is `rs autopilot best-friend ...`, so refreshed memory naturally turns into one checked launch packet for the next AI. With `--best-friend`, it also writes `best-friend.json` plus optional Markdown from the just-refreshed memory report without recursively re-running memory consolidation.

```powershell
rs autopilot remember .rs\autopilot\runs\starter-game --root .rs\autopilot\runs --format json
rs autopilot remember .rs\autopilot\runs\starter-game --root .rs\autopilot\runs --best-friend --format json
```

Use this after `demo-learn` when the next AI should inherit the creator's latest taste, constraints, canon, and operating rules without manually running four separate refresh commands. `remember` is non-mutating guidance consolidation only: it does not implement feedback, approve apply, mutate Studio, prove live playtest, publish, or mark the request complete. Run the recommended `best-friend` next action when a fresh or context-compacted AI should immediately receive the read order, checked opening reply, first safe action, and stop boundaries; use `--best-friend` when that launch packet should be produced in the same receipt.

### `rs autopilot advance`

Executes exactly one navigator-selected safe offline action. It refreshes a before-state navigator card, accepts only an `rs autopilot act <run-dir> --command "<safe offline command>"` wrapper from the navigator, refuses arbitrary shell text, writes `advance.json` plus `advance.md`, executes the inner command through `act`, then refreshes `navigator.json` / `.md` after the action.

```powershell
rs autopilot advance .rs\autopilot\runs\starter-game --format json
rs autopilot advance .rs\autopilot\runs\starter-game --dry-run --format json
```

Use this when the AI should make one bounded move without copying terminal text. `advance` never applies, publishes, uploads, runs live Studio setup, or crosses a mutation boundary. After one action it stops, points back to `navigator`, and requires proof or claim-check evidence before reporting completion.

### `rs autopilot act`

Executes one whitelisted offline Autopilot action selected from `diagnosis.json`, `cycle.json`, `work-order.json`, or `agenda.json`, then writes `act.json` plus `act.md`. It never shells out to arbitrary text; supported commands are dispatched through internal handlers such as `critique`, `work-check`, `cycle`, `diagnose`, `proof`, `acceptance`, `fulfillment`, `completion-audit`, `deliver`, `model-pack`, `task-pack`, `best-friend`, `best-friend-check`, `best-friend-rescue`, `best-friend-mentor`, `best-friend-pilot`, `best-friend-control`, `best-friend-operate`, `best-friend-runner`, `first-turn`, `best-friend-loop`, `best-friend-reply`, `best-friend-turn`, `best-friend-session`, `wow-session`, `best-friend-arc`, `squad-pack`, `squad-review`, `wow-plan`, `moment-pack`, `moment-sprint`, `moment-decision`, `creator-demo`, `demo-response`, `demo-loop`, `demo-session`, `demo-check`, `demo-learn`, `remember`, `satisfy`, `promise-loop`, `pursue`, `agenda`, `retrospect`, `playbook`, `self-check`, `claim-check`, `respond`, and `bundle`.

Use this when the AI has a safe next command but should not manually copy/paste terminal fragments. `act --source agenda` executes the first agenda command that has a safe internal handler. `act` refuses live or mutating boundaries such as `apply`, `live-gate`, `ready`, `setup`, bridge commands, upload, smoke, and plugin repair. After most successful run-local actions, it refreshes `cycle.json`, `diagnosis.json`, and the bundle so the next model can continue from artifacts.

```powershell
rs autopilot act .rs\autopilot\runs\starter-game --format json
rs autopilot act .rs\autopilot\runs\starter-game --dry-run --format json
rs autopilot act .rs\autopilot\runs\starter-game --source agenda --format json
rs autopilot act .rs\autopilot\runs\starter-game --command "rs autopilot critique --run-dir .rs\autopilot\runs\starter-game --format json"
```

### `rs autopilot loop`

Runs the guarded offline cycle/action loop until the run becomes `readyToReport`, an action is blocked, dry-run selection stops, or `--max-steps` is reached. It writes `loop.json` plus `loop.md`, records each cycle status, each act receipt, selected command kind, artifacts touched, and the final safe next action.

Use this when an AI should make bounded offline progress without manually alternating between `cycle`, `act`, and `diagnose`. It still refuses live or mutating commands through `act`; it does not apply to Studio, run smoke tests, upload assets, install plugins, or claim playtest/publish/production success.

```powershell
rs autopilot loop .rs\autopilot\runs\starter-game --max-steps 3 --format json
rs autopilot loop .rs\autopilot\runs\starter-game --dry-run --format json
```

### `rs autopilot judge`

Writes an honest readiness judgment for one run. It combines certification, gameplay critique, bundle verification, playtest-plan presence, and expected artifacts, then emits `judgment.json` plus `judgment.md` with a verdict such as `needsGameplayWork`, `readyForLiveApply`, or `appliedNeedsLiveProof`.

```powershell
rs autopilot judge .rs\autopilot\runs\shop --format json
rs autopilot judge .rs\autopilot\runs\tycoon --out .rs\reviews\tycoon-judgment.json
```

Use this before claiming a run is demo-ready. The judgment is deliberately conservative: even a playable, certified run is not production-ready until live apply, validation, rollback, and `playtest-result.json` evidence have been recorded.

### `rs autopilot critique`

Reviews an Autopilot run as a gameplay slice instead of only a technical plan. It scores core loop, player interaction, rewards, progression, feedback UI, onboarding, server authority, verification gates, and agent continuity, then writes `gameplay-critique.json` plus `gameplay-critique.md`.

```powershell
rs autopilot critique --run-dir .rs\autopilot\runs\starter-game --format json
rs autopilot critique --plan .rs\autopilot\runs\shop\plan.json --markdown .rs\reviews\shop-critique.md
```

Use this before live apply when an AI has produced a technically valid plan but might still be missing the playable loop, onboarding, reward path, or feedback layer that makes the result feel like a game.

### `rs autopilot playtest`

Writes a recipe-aware live playtest checklist for a run or plan. It turns recipes such as `starterShop`, `tycoonCore`, `obbyCheckpoint`, `enemyEncounter`, and `saveDataScaffold` into concrete Studio play steps, expected evidence, and follow-up commands.

```powershell
rs autopilot playtest --run-dir .rs\autopilot\runs\starter-game --format json
rs autopilot playtest --plan .rs\autopilot\runs\shop\plan.json --markdown .rs\reviews\shop-playtest.md
```

Use this after critique and before claiming the generated slice works in play mode. The command does not mutate Studio; it records what must be proven once `ready` and approved `apply` have succeeded.

### `rs autopilot simulate`

Writes a static player-journey simulation for a run or plan. It checks whether the planned artifacts imply a complete dry-play path: arrival, first interaction, reward feedback, progression, UI feedback, server authority, and evidence handoff. It does not contact Studio or claim runtime success.

```powershell
rs autopilot simulate --run-dir .rs\autopilot\runs\starter-game --format json
rs autopilot simulate --plan .rs\autopilot\runs\shop\plan.json --markdown .rs\reviews\shop-simulation.md
```

Use this before live apply when an AI needs a quick answer to "could a player understand and exercise this slice from static evidence?" Missing beats become concrete patch or playtest actions.

### `rs autopilot graph`

Writes an AI-readable feature graph for a run or plan. It connects recipes, created instances, generated sources, script targets, remotes, UI surfaces, and verifier operations into nodes and edges so a model can reason about system structure without rereading the entire plan.

```powershell
rs autopilot graph --run-dir .rs\autopilot\runs\starter-game --format json
rs autopilot graph --plan .rs\autopilot\runs\shop\plan.json --markdown .rs\reviews\shop-graph.md
```

Use this before planner handoff or repair work when the next AI needs to understand which scripts write to which instances, which remotes are referenced, and where verification gates sit in the feature stack.

### `rs autopilot balance`

Writes an economy balance report for a run or plan. It extracts currency names, reward values, prices, upgrade costs, starter balances, first-purchase pacing, and tuning findings from the tuned manifest and generated Luau. It does not playtest the economy; it gives the AI a static pacing gate before asking for live apply.

```powershell
rs autopilot balance --run-dir .rs\autopilot\runs\starter-game --format json
rs autopilot balance --plan .rs\autopilot\runs\shop\plan.json --markdown .rs\reviews\shop-balance.md
```

Use this with `simulate` and `graph` when a run has shop, coin, tycoon, reward, or upgrade systems. Blockers such as prices without rewards become concrete retune or recipe-patch commands.

### `rs autopilot impact`

Writes a mutation blast-radius map for a run or plan. It groups operations by service, script, remote, asset, delete, and cloud/persistence surface, then reports approval pressure, rollback requirements, touched paths, generated sources, and next commands before any live Studio mutation.

```powershell
rs autopilot impact .rs\autopilot\runs\starter-game --format json
rs autopilot impact --plan .rs\autopilot\runs\shop\plan.json --markdown .rs\reviews\shop-impact.md
```

Use this before approval, `live-gate`, or apply so the AI can explain exactly which Roblox services and runtime contracts are in the blast radius.

### `rs autopilot contracts`

Writes a static RemoteEvent and RemoteFunction contract map for a run or plan. It links generated scripts to remote endpoints, classifies client calls, server handlers, server emits, and client listeners, and flags one-sided or weakly validated contracts before live apply.

```powershell
rs autopilot contracts .rs\autopilot\runs\starter-game --format json
rs autopilot contracts --plan .rs\autopilot\runs\shop\plan.json --markdown .rs\reviews\shop-contracts.md
```

Use this with `graph` and `impact` when an AI needs to understand the generated client/server API surface instead of guessing from Luau source.

### `rs autopilot authority`

Writes a static server-authority and exploit-surface audit for a run or plan. It checks generated script sides, flags client-side DataStore access, client-owned player-state writes, profile mutation hooks, and imports remote-contract warnings into one authority score before live apply.

```powershell
rs autopilot authority .rs\autopilot\runs\starter-game --format json
rs autopilot authority --plan .rs\autopilot\runs\shop\plan.json --markdown .rs\reviews\shop-authority.md
```

Use this with `contracts`, `impact`, and `audit-sources` when an AI needs to prove gameplay-critical state is server-owned and remote contracts are reviewable before approval or mutation.

### `rs autopilot ux`

Writes a static player-facing UX audit for a run or plan. It maps generated UI and world prompt surfaces, checks visible player controls, interaction handlers, feedback text updates, onboarding copy, and readable text sizing evidence.

```powershell
rs autopilot ux .rs\autopilot\runs\starter-game --format json
rs autopilot ux --plan .rs\autopilot\runs\shop\plan.json --markdown .rs\reviews\shop-ux.md
```

Use this with `simulate`, `graph`, and `playtest` when an AI needs to know whether a player can understand what to do before asking for live apply.

### `rs autopilot copy-deck`

Writes a player-facing copy deck for a run or plan. It extracts static UI text from plan properties plus generated Luau strings from buttons, labels, prompts, status messages, and feedback updates, then reports dynamic strings and localization readiness.

```powershell
rs autopilot copy-deck .rs\autopilot\runs\starter-game --format json
rs autopilot copy-deck --plan .rs\autopilot\runs\shop\plan.json --markdown .rs\reviews\shop-copy.md
```

Use this with `ux`, `playtest`, and `planner-pack` when an AI needs to tune, localize, or explain every generated player-facing string without spelunking through source files.

### `rs autopilot performance`

Writes a static performance-budget audit for a run or plan. It estimates planned instance and script counts, generated source size, loop/frame-step patterns, wait/delay calls, remote traffic references, async fanout, and persistence references before live apply.

```powershell
rs autopilot performance .rs\autopilot\runs\starter-game --format json
rs autopilot performance --plan .rs\autopilot\runs\shop\plan.json --markdown .rs\reviews\shop-performance.md
```

Use this with `impact`, `authority`, and `playtest` when an AI needs to prove the slice is likely lightweight enough to apply and observe in Play Solo.

### `rs autopilot accessibility`

Writes a static accessibility audit for a run or plan. It checks generated UI surfaces for scalable text, touch-sized controls, readable text/background contrast, input affordance signals, mouse-only handlers, and motion-sensitive generated source patterns.

```powershell
rs autopilot accessibility .rs\autopilot\runs\starter-game --format json
rs autopilot accessibility --plan .rs\autopilot\runs\shop\plan.json --markdown .rs\reviews\shop-accessibility.md
```

Use this with `ux`, `copy-deck`, and `playtest` when an AI needs to know whether generated UI is inclusive enough to review before live apply.

### `rs autopilot policy`

Writes a static Roblox policy/safety audit for a run or plan. It scans generated plan text and Luau for purchases, randomized reward language, persistence, teleports, external HTTP, chat/user content, personal data requests, and off-platform links.

```powershell
rs autopilot policy .rs\autopilot\runs\starter-game --format json
rs autopilot policy --plan .rs\autopilot\runs\shop\plan.json --markdown .rs\reviews\shop-policy.md
```

Use this with `privacy`, `audit-sources`, and `approval` when an AI needs to surface policy-sensitive Roblox features before asking a creator to apply or publish a generated slice.

### `rs autopilot style-guide`

Writes a durable style bible for a run or plan. It infers theme, genre, tone, palette, visual rules, UI rules, copy rules, audio rules, and reusable asset prompts from the creator request, recipe evidence, and plan.

```powershell
rs autopilot style-guide .rs\autopilot\runs\starter-game --format json
rs autopilot style-guide --plan .rs\autopilot\runs\shop\plan.json --markdown .rs\reviews\shop-style.md
```

Use this before `asset-brief`, `copy-deck`, `import-image`, `import-asset`, and `upload` when an AI needs future patches and assets to feel like one coherent Roblox game.

### `rs autopilot world-blueprint`

Writes a spatial world blueprint for a run or plan. It turns the creator request, recipes, plan paths, and style direction into zones, player route steps, interaction anchors, camera shots, and build rules.

```powershell
rs autopilot world-blueprint .rs\autopilot\runs\starter-game --format json
rs autopilot world-blueprint --plan .rs\autopilot\runs\shop\plan.json --markdown .rs\reviews\shop-world.md
```

Use this before `asset-brief`, `playtest`, live apply, screenshots, or thumbnail work when an AI needs to know where the game loop happens in the Roblox world instead of guessing from scripts and operation paths.

### `rs autopilot onboarding`

Writes a first-session onboarding plan for a run or plan. It maps recipes and world zones into the first 90 seconds of player steps, teaching prompts, feedback expectations, and proof checks.

```powershell
rs autopilot onboarding .rs\autopilot\runs\starter-game --format json
rs autopilot onboarding --plan .rs\autopilot\runs\shop\plan.json --markdown .rs\reviews\shop-onboarding.md
```

Use this before `playtest`, `evidence`, `record-playtest`, or creator-facing demo claims when an AI needs to prove the generated game teaches the player what to do.

### `rs autopilot showcase`

Writes a creator-facing showcase plan for a run or plan. It turns the style guide, world blueprint, onboarding steps, recipes, and proof requirements into screenshot targets, thumbnail direction, trailer clips, talking points, publish-prep checks, and exact evidence recording commands.

```powershell
rs autopilot showcase .rs\autopilot\runs\starter-game --format json
rs autopilot showcase --plan .rs\autopilot\runs\shop\plan.json --markdown .rs\reviews\shop-showcase.md
```

Use this before screenshots, thumbnails, creator reviews, `publish-review`, or demo summaries when an AI needs to show the generated Roblox slice clearly without overstating unverified live proof.

### `rs autopilot telemetry`

Writes an analytics, funnel, and retention measurement plan for a run or plan. It turns recipes, onboarding, balance evidence, generated systems, and privacy rules into anonymous event names, properties, funnels, retention hooks, product questions, and next commands.

```powershell
rs autopilot telemetry .rs\autopilot\runs\starter-game --format json
rs autopilot telemetry --plan .rs\autopilot\runs\shop\plan.json --markdown .rs\reviews\shop-telemetry.md
```

Use this before adding analytics code, tuning retention loops, or claiming player-learning readiness when an AI needs to know what the generated Roblox game should measure without collecting personal data.

### `rs autopilot monetization`

Writes a conservative commerce-readiness plan for a run or plan. It turns recipes, balance, policy, telemetry, and generated shop/progression surfaces into offer candidates, commerce UI moments, price-test ideas, review inputs, and trust guardrails. It never invents Roblox product IDs or treats commerce as live.

```powershell
rs autopilot monetization .rs\autopilot\runs\starter-game --format json
rs autopilot monetization --plan .rs\autopilot\runs\shop\plan.json --markdown .rs\reviews\shop-monetization.md
```

Use this before adding MarketplaceService code, developer products, game passes, shop experiments, or revenue claims. The output stays review-only until creator approval, configured Roblox product/game pass IDs, server receipt handling, Studio apply, and playtest proof exist.

### `rs autopilot social`

Writes a Roblox-native social and growth-safe plan for a run or plan. It turns recipes, world layout, onboarding, telemetry, and policy review into social loops, optional friend moments, community hooks, proof checks, and guardrails. It never auto-invites, spams chat, requires friends for core progress, or claims growth systems are live.

```powershell
rs autopilot social .rs\autopilot\runs\starter-game --format json
rs autopilot social --plan .rs\autopilot\runs\shop\plan.json --markdown .rs\reviews\shop-social.md
```

Use this before adding friend visits, parties, group challenges, badges, leaderboards, community events, or social copy. The output stays proof-gated until the solo loop is playable, optional social copy is policy-safe, telemetry is defined, Studio apply succeeds, and playtest evidence captures the social moment.

### `rs autopilot liveops`

Writes a live operations plan for a run or plan. It turns recipes, telemetry, social, monetization, showcase, policy, and asset evidence into update cadence, event hooks, experiments, proof gates, operating rules, and exact next commands. It never treats update plans as published, tested, or live.

```powershell
rs autopilot liveops .rs\autopilot\runs\starter-game --format json
rs autopilot liveops --plan .rs\autopilot\runs\shop\plan.json --markdown .rs\reviews\shop-liveops.md
```

Use this before planning content drops, weekend events, retention experiments, shop refreshes, community challenges, or update summaries. The output stays proof-gated until bundle verification, approval, live readiness, rollback, policy review, telemetry, and playtest evidence support the update claim.

### `rs autopilot persistence`

Writes a DataStore and save/load contract for a run or plan. It turns recipes, generated source, balance, authority, telemetry, and policy evidence into data models, server-authoritative flows, schema migrations, proof checks, safety guardrails, and exact next commands. It never claims player progress is saved until implementation and reload proof exist.

```powershell
rs autopilot persistence .rs\autopilot\runs\starter-game --format json
rs autopilot persistence --plan .rs\autopilot\runs\shop\plan.json --markdown .rs\reviews\shop-persistence.md
```

Use this before adding DataStore code, save-data scaffolds, player profiles, tycoon progress, inventory, quest progress, or shop entitlement persistence. The output stays proof-gated until API-services readiness, server authority, policy review, approved apply, reload playtest evidence, and rollback evidence exist.

### `rs autopilot asset-brief`

Writes an art, audio, UI, VFX, and thumbnail production brief for a run or plan. It scans recipes, plan operations, prompt text, generated Luau, the style guide, and the world blueprint, then emits concrete asset requests with generation prompts, acceptance checks, and import/upload command templates.

```powershell
rs autopilot asset-brief .rs\autopilot\runs\starter-game --format json
rs autopilot asset-brief --plan .rs\autopilot\runs\shop\plan.json --markdown .rs\reviews\shop-assets.md
```

Use this with `style-guide`, `world-blueprint`, `copy-deck`, `ux`, `import-image`, `import-asset`, `import-audio`, and `upload` when an AI needs to make a generated slice feel real instead of stopping at primitive parts and placeholder UI.

### `rs autopilot trace`

Writes a prompt-to-artifact traceability matrix for a run. It maps the creator prompt to expected deterministic recipes, confirms those recipes are present, checks generated source files, and verifies the core offline review artifacts such as preview, source audit, simulation, feature graph, balance, impact, contracts, authority, UX, copy deck, performance, accessibility, policy, style guide, world blueprint, onboarding, showcase, telemetry, monetization, social, liveops, persistence, asset brief, and bundle.

```powershell
rs autopilot trace .rs\autopilot\runs\starter-game --format json
rs autopilot trace .rs\autopilot\runs\shop "make a shop with coins" --markdown .rs\reviews\shop-trace.md
```

Use this before handoff, acceptance, or user-facing summaries when an AI needs to prove it covered the request rather than only proving that a plan exists.

### `rs autopilot refresh`

Rebuilds the derived offline review packet for an existing run in the correct order. It refreshes source audit, simulation, feature graph, balance, impact, contracts, authority, UX, copy deck, performance, accessibility, policy, style guide, world blueprint, onboarding, asset brief, showcase, telemetry, monetization, social, liveops, persistence, trace, bundle, handoff, certification, planner pack, gameplay critique, and playtest plan, then writes `refresh.json` plus `refresh.md`.

```powershell
rs autopilot refresh .rs\autopilot\runs\starter-game --format json
rs autopilot refresh .rs\autopilot\runs\starter-game --markdown .rs\reviews\starter-refresh.md
```

Use this after generated files, plans, or review artifacts change, or when an AI resumes a run and needs a current offline packet without remembering every regeneration command.

### `rs autopilot evidence`

Writes the live-proof collection kit for one run. It refreshes `playtest-plan.json`, creates an `evidence/` folder layout, and emits `evidence-kit.json` plus `evidence-kit.md` with scenario-specific evidence requirements, suggested screenshot/log/note paths, and exact `record-playtest` commands.

```powershell
rs autopilot evidence .rs\autopilot\runs\starter-game --format json
rs autopilot evidence .rs\autopilot\runs\starter-game --evidence-dir .rs\evidence\starter-game
```

Use this after approval planning and before live Play Solo validation. The kit does not claim success; it tells the next AI or human what to capture and how to record it after `ready` and approved apply succeed.

### `rs autopilot record-playtest`

Records the result of a live Studio playtest after the generated checklist has been run. The command writes `playtest-result.json` plus `playtest-result.md` with the result, evidence, notes, per-scenario status, live-apply proof state, blockers, and next actions.

```powershell
rs autopilot record-playtest .rs\autopilot\runs\starter-game --result passed --evidence "Play Solo cash upgraded after collector touch"
rs autopilot record-playtest .rs\autopilot\runs\starter-game --result failed --scenario tycoonCore=failed --note "upgrade button did not spend cash"
```

Passing results are accepted only when at least one evidence item is supplied and `certification.json` records `applied: true`. `judge` consumes this artifact, but still requires rollback evidence and a verified bundle before it can mark a run production-ready.

### `rs autopilot evidence-review`

Reviews recorded live evidence before the next AI step. It reads `playtest-plan.json`, `evidence-kit.json`, and `playtest-result.json`, then writes `evidence-review.json` plus `evidence-review.md` with scenario-level observations, repair hypotheses, safe-to-say claims, do-not-say claims, and next commands.

```powershell
rs autopilot evidence-review .rs\autopilot\runs\starter-game --format json
rs autopilot evidence-review .rs\autopilot\runs\starter-game --markdown .rs\reviews\starter-evidence-review.md
```

Use this after `record-playtest` and before `repair-plan`, `judge`, or `health`. The review does not mutate Studio and does not upgrade evidence into success; it explains what the evidence supports and what the AI must not claim.

### `rs autopilot health`

Writes the applied-run health gate. It refreshes proof, judgment, and rollback artifacts, then writes `health.json` plus `health.md` with pass/fail checks for `apply.json`, `validationAfter`, mandatory `regression` smoke, rollback artifact, live playtest result, and proof ledger.

```powershell
rs autopilot health .rs\autopilot\runs\starter-game --format json
rs autopilot health .rs\autopilot\runs\starter-game --markdown .rs\reviews\starter-health.md
```

Use this before saying an applied run is healthy. `appliedAndHealthy` requires a successful apply with no refused operations, zero validation failures after apply, passing regression smoke evidence, rollback proof, accepted live playtest proof, and production-ready proof. Missing smoke or validation evidence returns `needsHealthProof`.

### `rs autopilot repair-plan`

Turns a failed, blocked, or inconclusive `playtest-result.json` into a repair packet for the next AI agent. It writes `repair-plan.json` plus `repair-plan.md` with inferred incidents, affected recipes, evidence, safe commands, and a planner prompt that asks for a strict patch plan rather than free-form code.

```powershell
rs autopilot repair-plan .rs\autopilot\runs\starter-game --format json
rs autopilot repair-plan .rs\autopilot\runs\starter-game --out .rs\reviews\starter-repair.json
```

Use this after `record-playtest --result failed|blocked|inconclusive`. The command does not mutate Studio; it bridges live failure evidence back into the normal planner/adopt/certify workflow.

### `rs autopilot improve`

Creates an offline patch run from gameplay critique gaps. It re-critiques the source run, selects missing deterministic recipes such as `tycoonCore`, `collectibleCoin`, `saveDataScaffold`, or `npcInteraction`, writes a new patch run, then emits source audit, handoff, bundle, certification, planner pack, gameplay critique, playtest plan, and `improve.json`/`improve.md`.

```powershell
rs autopilot improve --run-dir .rs\autopilot\runs\shop --max-recipes 2 --format json
rs autopilot improve --run-dir .rs\autopilot\runs\shop --recipe tycoonCore --out .rs\autopilot\runs\shop-tycoon-patch
```

Use this when critique says a run is technically valid but not yet playable. The command does not contact Studio or apply changes; it turns the weakest design gap into the next reviewable run so an AI can iterate without parsing free-form advice.

### `rs autopilot compare`

Compares a baseline run against a candidate run. It reports gameplay score delta, verdict delta, recipe changes, Studio path changes, generated-file changes, bundle state, certification verdicts, blockers, warnings, and the next safe continuation command.

```powershell
rs autopilot compare --base-run .rs\autopilot\runs\shop --candidate-run .rs\autopilot\runs\shop-tycoon-patch --format json
rs autopilot compare --base-run .rs\autopilot\runs\a --candidate-run .rs\autopilot\runs\b --out .rs\reviews\compare.json
```

Use this after `improve`, `adopt-plan`, or competing planner outputs. It gives an AI model a machine-readable answer to "which run should I continue from?" instead of relying on folder names or vibes.

### `rs autopilot iterate`

Runs the offline improvement loop for an existing run. It critiques the current run, selects deterministic recipe patches, writes candidate step runs, compares each candidate against the current best, and stops when a playable slice is found or deterministic recipes are exhausted.

```powershell
rs autopilot iterate --run-dir .rs\autopilot\runs\shop --max-steps 3 --max-recipes 2 --format json
rs autopilot iterate --run-dir .rs\autopilot\runs\shop --out .rs\autopilot\iterations\shop-loop --smoke regression
```

The session writes `iteration.json` and `iteration.md` plus one folder per accepted candidate, such as `step-01-tycooncore`. Each candidate keeps the normal plan, preview, source audit, bundle, handoff, certification, critique, playtest, and comparison artifacts. Use this when an AI should keep improving safely without asking the user to manually run critique, improve, compare, and verify for every step.

### `rs autopilot sequence`

Writes an ordered apply sequence for multiple run folders. It summarizes each step's plan, recipes, gameplay verdict, bundle state, warnings, blockers, and exact apply command, then emits `sequence.json` plus `sequence.md`.

```powershell
rs autopilot sequence --run-dir .rs\autopilot\runs\shop --run-dir .rs\autopilot\runs\shop-tycoon-patch --format json
rs autopilot sequence --run-dir .rs\autopilot\iterations\shop-loop\step-01-tycooncore --out .rs\reviews\shop-sequence.json
```

Use this after `iterate` or after reviewing several patch runs. The command does not contact Studio or mutate anything; it gives an AI a durable, machine-readable order of operations before `ready` and approved `apply` commands.

### `rs autopilot architect`

Turns a creator prompt into a staged build architecture for AI agents. It writes `architect.json` plus `architect.md` with the inferred genre, player promise, recipe stack, phase-by-phase commands, validation gates, live-readiness requirements, and rollback/apply path. The phase list starts with `tune`, then composes from the generated manifest so AI agents do not skip recipe-specific naming and economy review. It does not contact Studio or mutate anything.

```powershell
rs autopilot architect "Make a tycoon game with droppers, upgrades, and admin tools" --smoke regression --format json
```

This is the planning layer an AI should use before composing a large feature stack from a vague user request.

### `rs autopilot kickoff`

Runs the full offline AI-safe startup sequence for a prompt: write architecture, generate a tuned compose manifest, compose the deterministic plan from that manifest, seal a handoff bundle, certify the run, and write project memory for immediate resumption. It still does not contact Studio or mutate anything; it produces a ready-to-review packet that can be handed to another agent or used after `ready` passes.

```powershell
rs autopilot kickoff "Make a tycoon game with droppers and upgrades" --smoke regression --out .rs\autopilot\runs\tycoon-kickoff --format json
```

Expected artifacts include `session.json`, `pitch.json`, `storyboard.json`, `proposal.json`, optional `companion.json`, optional `companion-proposal.json`, optional `companion-setup.json`, optional `selection.json`, optional `launch.json`, `drive.json`, `live-gate.json`, optional `rehearsal.json`, `closeout.json`, `timeline.json`, `start.json`, `cockpit.json`, `agent-capsule.json`, optional `model-pack.json`, optional `task-pack.json`, optional `best-friend.json`, optional `best-friend-self-check.json`, optional `best-friend-turn.json`, optional `best-friend-session.json`, optional `wow-session.json`, optional `best-friend-arc.json`, optional `squad-pack.json`, optional `squad-review.json`, optional `wow-plan.json`, optional `moment-pack.json`, optional `moment-sprint.json`, optional `moment-decision.json`, optional `creator-demo.json`, optional `demo-response.json`, optional `demo-loop.json`, optional `demo-session.json`, optional `demo-check.json`, optional `demo-reply.json`, optional `demo-learn.json`, optional `remember.json`, `review-pack.json`, optional `studio-review.json`, optional `publish-prep.json`, optional `feedback.json`, optional `feedback-patch.json`, optional `feedback-planner-pack.json`, optional `claim-check.json`, optional `response.json`, optional `inbox.json`, optional `handle.json`, optional `conversation.json`, optional `chat.json`, optional `decisions.json`, optional `alignment.json`, optional `journal.json`, optional `opportunities.json`, optional `work-order.json`, optional `work-check.json`, optional `cycle.json`, optional `diagnosis.json`, optional `command-guard.json`, optional `self-check.json`, optional `act.json`, optional `loop.json`, optional `agenda.json`, optional `sprint.json`, optional `retrospective.json`, optional `ai-playbook.json`, optional `capability-atlas.json`, `proof.json`, `health.json`, `acceptance.json`, optional `fulfillment.json`, optional `completion-audit.json`, optional `delivery.json`, optional `satisfy.json`, optional `promise-loop.json`, `rollback.json`, `approval.json`, `privacy.json`, `intake.json`, optional `survey.json`, optional `reconcile.json`, optional `scout.json`, `architect.json`, `tune.autopilot-compose.json`, `tune.md`, `plan.json`, `preview.json`, `simulation.json`, `simulation.md`, `feature-graph.json`, `feature-graph.md`, `balance.json`, `balance.md`, `impact.json`, `impact.md`, `contracts.json`, `contracts.md`, `authority.json`, `authority.md`, `ux.json`, `ux.md`, `copy-deck.json`, `copy-deck.md`, `performance.json`, `performance.md`, `accessibility.json`, `accessibility.md`, `policy.json`, `policy.md`, `style-guide.json`, `style-guide.md`, `world-blueprint.json`, `world-blueprint.md`, `onboarding.json`, `onboarding.md`, `showcase.json`, `showcase.md`, `telemetry.json`, `telemetry.md`, `monetization.json`, `social.json`, `social.md`, `liveops.json`, `liveops.md`, `persistence.json`, `persistence.md`, `asset-brief.json`, `asset-brief.md`, `trace.json`, `trace.md`, `handoff.json`, `certification.json`, `gameplay-critique.json`, `playtest-plan.json`, `project-memory.json`, optional `evidence-kit.json`, optional `playtest-result.json`, optional `repair-plan.json`, optional `user-brief.json`, `planner-pack.json`, `kickoff.json`, companion Markdown files, generated Luau under `generated/`, and live proof files under `evidence/`.

### `rs autopilot audit-sources`

Scans generated Luau before live apply. It reports blocker findings for dynamic code execution, external numeric `require` calls, direct HTTP usage, environment/debug introspection, missing generated files, unsafe artifact paths, and secret-like markers. It reports warnings for DataStore, Marketplace, and Teleport service usage that require live-place review.

```powershell
rs autopilot audit-sources .rs\autopilot\runs\tycoon-kickoff --format json
```

`kickoff` writes `source-audit.json` and `source-audit.md` before creating the bundle, and `certify` includes a source-audit gate.

### `rs autopilot planner-pack`

Writes the provider-ready packet an AI model should read before proposing a custom plan. The packet includes the strict plan contract, supported operation kinds, recipe catalog, redacted context, redacted run artifacts, safety rules, expected JSON-only model output, and validation commands. It never calls an external AI provider and never mutates Studio.

```powershell
rs autopilot planner-pack "Add a shop and coin loop" --context .rs\autopilot\context\context.json --format json
rs autopilot planner-pack --run-dir .rs\autopilot\runs\tycoon-kickoff --out .rs\autopilot\runs\tycoon-kickoff\planner-pack.json
```

Use this when a deterministic recipe is not enough and an external planner needs bounded, redacted facts instead of free-form authority.

### `rs autopilot adopt-plan`

Adopts a strict `rs.autopilot.plan.v1` JSON object returned by an AI planner into a normal Autopilot run folder. The command rejects unsupported top-level fields, operation fields, embedded generated source, `runLuau`, and `sourcePath` values outside `generated/`. It copies referenced generated files, writes preview/source-audit/handoff/bundle/certification/planner-pack artifacts, and exits non-zero if certification finds blockers.

```powershell
rs autopilot adopt-plan --plan .rs\ai-output\plan.json --source-root .rs\ai-output --out .rs\autopilot\runs\ai-shop --format json
```

This is the intake gate for model-backed planning: AI output becomes useful only after the CLI materializes, audits, previews, bundles, and certifies it through the same path as deterministic plans.

### `rs autopilot certify`

Writes a deterministic go/no-go certificate for one run. It checks plan schema, generated sources, generated Luau source safety, preview safety, preview integrity, bundle verification, handoff state, apply result, and live-readiness requirements, then writes `certification.json` plus `certification.md`.

```powershell
rs autopilot certify .rs\autopilot\runs\starter-game --format json
rs autopilot certify .rs\autopilot\runs\starter-game --out .rs\handoffs\starter-game.certification.json
```

### `rs autopilot bundle`

Writes a hashed handoff manifest for a run folder. `bundle.json` records required artifacts, generated-file hashes, current coach status, blockers, warnings, and next actions so another agent can audit or resume the run.

```powershell
rs autopilot bundle .rs\autopilot\runs\starter-game --format json
```

### `rs autopilot verify-bundle`

Verifies a `bundle.json` manifest against the run folder before another agent or CI job trusts it. The command checks every recorded artifact path, byte count, and hash, reports extra local files, and exits non-zero when required artifacts are missing or drifted.

```powershell
rs autopilot verify-bundle --run-dir .rs\autopilot\runs\starter-game --format json
rs autopilot verify-bundle --bundle .rs\handoffs\starter-game.bundle.json --run-dir .rs\autopilot\runs\starter-game
```

### `rs autopilot publish-prep`

Writes a Roblox-facing launch dossier without publishing or uploading anything. It reads the run plan plus bundle, showcase, policy, privacy, reconciliation, health, closeout, proof, acceptance, rollback, and asset-brief artifacts. Output includes `publish-prep.json` plus `publish-prep.md` with store title/description drafts, update notes, store asset needs, release checklist, safe claims, forbidden claims, blockers, and next commands.

Use this when an AI is ready to help the creator move from a built slice toward Roblox release work. `needsLiveProof` means store copy can be reviewed, but live proof still blocks release-candidate claims. `releaseCandidate` means the local publish-prep checklist passes; it still does not mean the game was published.

```powershell
rs autopilot publish-prep .rs\autopilot\runs\starter-game --format json
rs autopilot publish-prep .rs\autopilot\runs\starter-game --markdown .rs\reviews\starter-publish-prep.md
```

### `rs autopilot feedback`

Turns creator, playtester, or AI review notes into an offline patch triage packet. It writes `feedback.json` plus `feedback.md` with categorized feedback items, severity, scope, confidence, artifact evidence, clarification questions, patch lanes, safe claims, forbidden claims, blockers, and exact next commands.

Use this after a creator reviews a proposal, Studio review panel, showcase, playtest, or publish-prep dossier. `readyForPatch` means notes were specific enough to route into existing offline review/patch commands. `needsFeedback` or `needsClarification` exits non-zero so an AI does not pretend vague or missing feedback was handled.

```powershell
rs autopilot feedback .rs\autopilot\runs\starter-game --note "shop button is confusing" --note "thumbnail needs brighter colors" --format json
rs autopilot feedback .rs\autopilot\runs\starter-game --source playtester --note "prices feel too high" --markdown .rs\reviews\starter-feedback.md
```

### `rs autopilot feedback-patch`

Converts `feedback.json` into a strict AI patch work order. It writes `feedback-patch.json` plus `feedback-patch.md`, and by default also writes `feedback-planner-pack.json` plus `feedback-planner-pack.md` with a feedback-specific prompt for a model to produce one strict `rs.autopilot.plan.v1` patch.

Use this when feedback has moved past triage and the next model needs precise instructions. `readyForPlanner` means the notes are concrete enough to hand to an AI planner; it still does not mean the patch exists, was adopted, was certified, or changed Studio. Missing feedback or unresolved clarification exits non-zero.

```powershell
rs autopilot feedback-patch .rs\autopilot\runs\starter-game --format json
rs autopilot feedback-patch .rs\autopilot\runs\starter-game --planner-pack .rs\reviews\starter-feedback-planner-pack.json
```

### `rs autopilot claim-check`

Checks proposed creator-facing claims against the run's proof, acceptance, health, rollback, publish-prep, feedback, feedback-patch, decisions, alignment, journal, opportunities, work order, review, brief, and bundle evidence. It writes `claim-check.json` plus `claim-check.md` with one verdict per claim, cited artifacts, blockers, warnings, safe rewrites, and the next command to run before responding.

Use this immediately before an AI tells the creator what happened. `readyToSay` means every supplied claim is backed by current artifacts. `needsRewrite` means the claim is close but too broad. `blocked` exits non-zero for unsupported claims such as published, live-applied, production-ready, playtested, or rollback-safe statements without matching evidence.

```powershell
rs autopilot claim-check .rs\autopilot\runs\starter-game --claim "Feedback has been converted into a feedback patch planner work order." --format json
rs autopilot claim-check .rs\autopilot\runs\starter-game --claim "The game is production-ready and published on Roblox." --format json
```

### `rs autopilot respond`

Composes a creator-facing response from `user-brief.json` and `claim-check.json`. It writes `response.json` plus `response.md`, refreshes the brief and claim check, includes the exact checked claims, and exits non-zero unless every claim is safe to send.

Use this as the final step before an AI reports status to the creator. With explicit `--claim` values, the response is safe only for those exact claims. Without claims, it drafts from the current safe-to-say list. `readyToSend` means the message may be used; `blocked` or `needsRewrite` means the model must use the safe rewrite instead.

```powershell
rs autopilot respond .rs\autopilot\runs\starter-game --claim "The run has a structurally valid Autopilot plan." --format json
rs autopilot respond .rs\autopilot\runs\starter-game --claim "The game is production-ready and published on Roblox." --format json
```

### `rs autopilot decision`

Records creator decisions, constraints, rejections, and notes in `decisions.json` plus `decisions.md` without mutating Studio or granting live approval. It accepts repeated `--decision`, `--constraint`, `--rejection`, and `--note` values, records active constraints and rejected directions, refreshes the bundle, and feeds safe decision claims into `claim-check` and `respond`.

Use this after the creator chooses a direction or rules out an option. `recorded` means the ledger is usable for future planning. `needsDecision` means no concrete entry was supplied. `needsApprovalGate` exits non-zero when an entry tries to approve live apply, publish, upload, rollback, release, or production work; those must go through `approval` and `live-gate`.

```powershell
rs autopilot decision .rs\autopilot\runs\starter-game --decision "Use the cozy shop direction" --constraint "Keep combat nonviolent" --rejection "Do not add gacha" --format json
rs autopilot decision .rs\autopilot\runs\starter-game --decision "Go ahead and apply this in live Studio" --format json
```

### `rs autopilot align`

Checks `plan.json` and generated sources against `decisions.json` before the next AI continues. It writes `alignment.json` plus `alignment.md`, reports active constraints, detects obvious rejected-direction terms in the plan, refreshes the bundle, and feeds one safe static-audit claim into `claim-check`.

Use this after `decision` and before another planner/adopt/apply loop. `aligned` means no rejected-direction terms were detected. `blocked` exits non-zero when a rejected option appears in the plan or generated source. `needsPlan` and `needsDecisions` tell the agent which artifact is missing. Alignment is not implementation proof and never grants apply or publish approval.

```powershell
rs autopilot align .rs\autopilot\runs\starter-game --format json
rs autopilot align .rs\autopilot\runs\starter-game --decisions .rs\reviews\creator-decisions.json --markdown .rs\reviews\starter-alignment.md
```

### `rs autopilot journal`

Records AI work notes, attempted commands, results, and evidence pointers in `journal.json` plus `journal.md`. It summarizes existing run artifacts, refreshes the bundle, and feeds only the safe continuity claim into `claim-check`.

Use this before handing a run to another AI session or after a long investigation. `recorded` means there is at least one note or existing artifact to preserve. `needsEntry` exits non-zero when the run has no notes and no artifacts. Journal entries are memory aids, not proof: logged commands do not count as passed checks, and live mutation terms route the agent back to approval/live-gate artifacts.

```powershell
rs autopilot journal .rs\autopilot\runs\starter-game --entry "Aligned decisions and found no drift" --command "rs autopilot align .rs\autopilot\runs\starter-game --format json" --result "alignment passed" --evidence alignment.json --format json
rs autopilot journal .rs\autopilot\runs\starter-game --command "rs autopilot apply --plan .rs\autopilot\runs\starter-game\plan.json" --result "failed: Studio was not connected" --format json
```

### `rs autopilot reconcile`

Compares a run's `plan.json` with live Studio evidence from `context.json` or `survey.json`. It writes `reconcile.json` plus `reconcile.md` with matched planned paths, missing paths, survey findings, safe zones, status, and exact next commands. Status values include `needsSurvey`, `aligned`, `plannedNotApplied`, `driftDetected`, and `needsReview`.

Use this after a live apply, after reopening Studio, or before a resumed AI claims that a run is present in the place. `driftDetected` means the run has `apply.json` evidence but planned paths are missing from the supplied Studio evidence, so the AI should refresh context or route to repair before more mutation.

```powershell
rs autopilot reconcile .rs\autopilot\runs\starter-game --context .rs\autopilot\context\context.json --format json
rs autopilot reconcile .rs\autopilot\runs\starter-game --survey .rs\autopilot\survey.json --markdown .rs\reviews\starter-reconcile.md
```

### `rs autopilot ready`

Checks the local bridge, polls connected Studio sessions, and verifies the protocol and plugin capabilities needed for live preview/apply. It attempts to start the local bridge automatically before polling, so an AI sees the real remaining blocker: missing Studio/plugin readiness rather than a generic bridge failure. This gives AI agents a bounded gate to run after asking the user to restart Studio or install the plugin.

```powershell
rs autopilot ready --studio "Demo Place" --timeout 90 --format json
```

### `rs autopilot setup`

Writes an AI-readable setup packet for bridge, Studio, and plugin readiness. It runs the same readiness gate as `ready`, writes `setup.json` plus `setup.md`, and converts blockers into exact next actions such as `rs install-plugin`, restarting Studio, `rs doctor --fix --format json`, and retrying `setup` or `ready`. With `--fix`, it reuses the existing install-plugin flow to build and copy the current plugin bundle first, then records the installed path, hash, and Studio windows that still need restart.

```powershell
rs autopilot setup --format json
rs autopilot setup --fix --format json
rs autopilot setup --studio "Demo Place" --out .rs\autopilot\setup.json
```

Use this when an AI needs to explain why live work is blocked and what the creator should do next. Without `--fix`, it only diagnoses. With `--fix`, it may change the local Roblox plugin bundle on disk, but it still does not mutate the open Studio place or claim live readiness until Studio reloads the plugin and `ready` passes.

### `rs autopilot plan`

Creates a plan without applying it.

```powershell
rs autopilot plan "Add an admin command panel" --studio "My Game" --scope game --out .rs\autopilot\runs\admin
```

Options:

- `--studio <selector>`: Studio name, UUID, or unique substring.
- `--scope <path>`: Studio path used for context and relative operations. Defaults to `game`.
- `--out <folder>`: Artifact directory.
- `--format text|json`: Output format.
- `--max-read-depth <n>`: Context depth for `read` and `snapshot`.
- `--include-scripts`: Include script source in planner context. Default should be false until redaction is solid.
- `--include-assets`: Include asset reference metadata.
- `--recipe <name>`: Use a deterministic built-in planner recipe.
- `--from-manifest <file>`: Build from a structured local request instead of natural language.

### `rs autopilot preview`

Runs preflight and dry-runs an existing plan.

```powershell
rs autopilot preview --studio "My Game" --plan .rs\autopilot\runs\admin\plan.json
```

`preview.json` includes a plan/generated-file integrity seal. `apply` verifies that seal when present and refuses to mutate Studio if `plan.json` or generated scripts changed after preview.

### `rs autopilot apply`

Applies an approved plan and verifies it.

```powershell
rs autopilot apply --studio "My Game" --plan .rs\autopilot\runs\admin\plan.json --yes --validate
```

Options:

- `--yes`: Required for mutation.
- `--rollback-on-error`: Restore the rollback package or plugin snapshot when application fails.
- `--force`: Allow changes to non-owned instances only when the plan's risk policy allows it.
- `--only <kind>`: Apply only matching operation kinds.
- `--exclude <kind>`: Exclude operation classes such as `scripts`, `assets`, or `deletes`.
- `--validate`: Run validation after apply.
- `--smoke <suite>`: Run a smoke suite after apply when available.

### `rs autopilot report`

Prints or exports a prior run report.

```powershell
rs autopilot report .rs\autopilot\runs\admin --format markdown
```

## Plan Schema

Autopilot plans are JSON files. They must be versioned, deterministic, and safe to validate without contacting an AI provider.

```json
{
  "schemaVersion": "rs.autopilot.plan.v1",
  "id": "autopilot-20260516-shop",
  "createdAt": "2026-05-16T00:00:00Z",
  "request": {
    "prompt": "Add a starter shop UI with purchase remotes and server-side validation",
    "scope": "game",
    "studio": "My Game"
  },
  "risk": {
    "level": "review",
    "requiresApproval": true,
    "destructive": false,
    "touchesScripts": true,
    "touchesCloudAssets": false
  },
  "preconditions": [
    {
      "type": "pathExists",
      "path": "ReplicatedStorage"
    },
    {
      "type": "pathExists",
      "path": "ServerScriptService"
    }
  ],
  "operations": [
    {
      "id": "op-001",
      "kind": "createInstance",
      "parentPath": "ReplicatedStorage",
      "className": "Folder",
      "name": "Shop",
      "ownership": "rs"
    },
    {
      "id": "op-002",
      "kind": "createInstance",
      "parentPath": "ReplicatedStorage.Shop",
      "className": "RemoteEvent",
      "name": "PurchaseRequested",
      "ownership": "rs"
    },
    {
      "id": "op-003",
      "kind": "upsertScript",
      "path": "ServerScriptService.ShopServer",
      "className": "Script",
      "sourcePath": "generated/ShopServer.server.lua",
      "ownership": "rs"
    },
    {
      "id": "op-004",
      "kind": "validate",
      "path": "game",
      "rules": ["refs", "assets", "duplicates"]
    }
  ],
  "artifacts": {
    "generatedFilesRoot": "generated",
    "report": "report.md"
  }
}
```

### Operation Kinds

Initial operation kinds:

- `createInstance`: Create a Studio instance with name, class, parent, attributes, tags, and primitive properties.
- `setProperty`: Set a supported property on an owned or explicitly approved instance.
- `setAttribute`: Set one attribute.
- `setTags`: Replace tag set on an instance.
- `deleteInstance`: Delete an owned instance only from `danger` plans with `destructive=true`; live apply requires `--force --rollback-on-error`.
- `upsertScript`: Create or update `Script`, `LocalScript`, or `ModuleScript` source.
- `upsertFiles`: Delegate to the existing file upsert path.
- `importImage`: Delegate to the PNG importer or UI pack importer.
- `importAsset`: Delegate to local mesh import.
- `importUploaded`: Create instances from known Roblox asset IDs.
- `uploadAsset`: Explicit Open Cloud upload with profile and wait controls.
- `packageImport`: Import a prepared package with conflict mode.
- `applyPlan`: Delegate a diff-style fix plan to `/apply-plan`.
- `validate`: Run `validate`.
- `deps`: Capture dependency graph evidence for the touched scope.
- `publishCheck`: Run validation and dependency checks as a share/publish readiness gate.
- `repairTool`: Run `repair-tool`.
- `snapshot`: Capture a subtree summary.
- `smoke`: Run a smoke suite.

Deferred operation kinds:

- `replacePackage`: Only after package update conflict policies are proven.
- `runLuau`: Only for reviewed, constrained maintenance scripts. This should not be in MVP.

## Safety Model

### Risk Levels

- `safe`: Creates new owned instances or validates state. No existing user content is changed.
- `review`: Updates owned instances, writes scripts, imports assets, or changes non-destructive properties.
- `danger`: Deletes instances, changes non-owned content, force-overwrites scripts, or modifies cloud assets.
- `refused`: Requests secrets, broad destructive edits, external code execution, or unsupported Roblox operations.

MVP should automatically apply only `safe` operations when `--yes` is present. `review` requires a visible preview. `danger` requires `--force` and should remain off by default. `refused` can never be applied.

### Ownership Rules

Autopilot must respect existing ownership attributes:

- `rsSourceId`
- `rsPackageId`
- `rsImportedAt`
- `rsManagedBy`

Default policy:

- New instances are stamped as `rsManagedBy = "rs.autopilot"`.
- Owned instances can be updated by plan operations.
- Non-owned existing instances can be read and referenced but not mutated without `--force`.
- Deletes are refused unless the target is owned and a rollback artifact exists.

### Approval Rules

- `plan` and `preview` must not mutate Studio.
- `apply` must require `--yes`.
- `--apply` shorthand must also require `--yes`.
- Script writes must be listed separately from instance/property changes.
- Cloud upload operations must name the profile and asset file before approval.
- Any operation that cannot be dry-run accurately must be marked `review` or `danger`.

## Planner Design

### Deterministic Planner First

The first implementation should support structured manifests and deterministic recipes before AI integration. This makes the execution engine testable.

Example manifest:

```json
{
  "kind": "starterShop",
  "name": "Shop",
  "currencyName": "Coins",
  "items": [
    { "id": "speed_boost", "displayName": "Speed Boost", "price": 100 },
    { "id": "double_jump", "displayName": "Double Jump", "price": 250 }
  ]
}
```

This can produce the same plan every time and exercise the full preview/apply/validate/rollback pipeline.

### AI-Backed Planner Later

The AI-backed planner should be isolated behind a small provider interface. Its only accepted output is a strict `rs.autopilot.plan.v1` JSON plan plus short rationale fields. The CLI validates the JSON, normalizes operations, rejects unsupported fields, and then runs the same preview/apply path as deterministic plans.

Planner input should include:

- User prompt.
- Selected Studio name and scope.
- Sanitized `snapshot`.
- Optional bounded `read` output.
- Validation diagnostics.
- Available capabilities and protocol version.
- Existing command inventory and operation schema.

Planner input must not include:

- API keys or auth profile secrets.
- Full local paths unless needed for an explicit file operation.
- Entire project source by default.
- Hidden command output that contains credentials.

## Artifact Layout

Every run writes a durable folder:

```text
.rs/
  autopilot/
    runs/
      20260516-154233-shop/
        request.json
        context.json
        preflight.json
        plan.json
        preview.json
        validation-before.json
        validation-after.json
        diff-before-after.json
        rollback.rspkg
        report.md
        generated/
          ShopServer.server.lua
          ShopClient.client.lua
          ShopConfig.module.lua
```

Rules:

- `request.json` preserves the user-facing request and CLI options.
- `context.json` is sanitized and redacted.
- `plan.json` is the exact plan applied or previewed.
- `rollback.rspkg` is written before mutation when rollback is possible.
- `report.md` is the human-readable summary for review or pull requests.
- Generated scripts should live under `generated/` and be referenced by plan operations.

## Report Format

`report.md` should include:

- Run ID, timestamp, CLI version, plugin version, protocol version.
- Studio selector and resolved Studio session.
- User request.
- Preconditions and their pass/fail state.
- Operation summary by kind and risk.
- Changed paths.
- Validation before and after.
- Smoke results, if requested.
- Rollback artifact path or reason rollback was unavailable.
- Refused operations and exact reasons.
- Follow-up manual checks, if any.

## Validation Strategy

### Unit Tests

- Plan schema round trips.
- Unsupported operation rejection.
- Risk classification for safe, review, danger, and refused operations.
- Manifest-to-plan deterministic output.
- Redaction of API keys, auth profiles, and environment-looking values.
- Artifact path normalization.

### Integration Tests Without Studio

- `rs autopilot plan --from-manifest` writes the full artifact set.
- `rs autopilot preview --plan` rejects invalid schema.
- `rs autopilot apply --plan` refuses mutation without `--yes`.
- Plan operations lower into existing command request structs.
- `--only` and `--exclude` filter operation classes predictably.

### Live Studio Smoke

- Empty-place starter shop dry-run creates no instances.
- Applying starter shop creates expected folders, remotes, scripts, and UI.
- Re-running the same plan is idempotent and does not duplicate instances.
- Validation after apply returns no fails for the touched scope.
- Forced failure during apply triggers rollback when `--rollback-on-error` is set.
- `history undo` or package restore can remove the created feature.

### Suggested Commands

```powershell
cargo fmt --check
cargo test -p rs
cargo build --release
rojo build plugin/default.project.json --output target\plugin-build-check.rbxmx
target\release\rs.exe autopilot plan --from-manifest examples\starter-shop.autopilot.json --out .rs\autopilot\runs\starter-shop
target\release\rs.exe autopilot style-guide .rs\autopilot\runs\starter-shop --format json
target\release\rs.exe autopilot world-blueprint .rs\autopilot\runs\starter-shop --format json
target\release\rs.exe autopilot onboarding .rs\autopilot\runs\starter-shop --format json
target\release\rs.exe autopilot showcase .rs\autopilot\runs\starter-shop --format json
target\release\rs.exe autopilot telemetry .rs\autopilot\runs\starter-shop --format json
target\release\rs.exe autopilot monetization .rs\autopilot\runs\starter-shop --format json
target\release\rs.exe autopilot social .rs\autopilot\runs\starter-shop --format json
target\release\rs.exe autopilot liveops .rs\autopilot\runs\starter-shop --format json
target\release\rs.exe autopilot persistence .rs\autopilot\runs\starter-shop --format json
target\release\rs.exe autopilot asset-brief .rs\autopilot\runs\starter-shop --format json
target\release\rs.exe autopilot evidence-review .rs\autopilot\runs\starter-shop --format json
target\release\rs.exe autopilot reconcile .rs\autopilot\runs\starter-shop --context .rs\autopilot\runs\starter-shop\context\context.json --format json
target\release\rs.exe autopilot publish-prep .rs\autopilot\runs\starter-shop --format json
target\release\rs.exe autopilot feedback .rs\autopilot\runs\starter-shop --note "shop button is confusing" --format json
target\release\rs.exe autopilot feedback-patch .rs\autopilot\runs\starter-shop --format json
target\release\rs.exe autopilot claim-check .rs\autopilot\runs\starter-shop --claim "Feedback has been converted into a feedback patch planner work order." --format json
target\release\rs.exe autopilot respond .rs\autopilot\runs\starter-shop --claim "Feedback has been converted into a feedback patch planner work order." --format json
target\release\rs.exe autopilot inbox "shop button is confusing" --run-dir .rs\autopilot\runs\starter-shop --format json
target\release\rs.exe autopilot handle "shop button is confusing" --run-dir .rs\autopilot\runs\starter-shop --format json
target\release\rs.exe autopilot conversation --run-dir .rs\autopilot\runs\starter-shop --format json
target\release\rs.exe autopilot chat "shop button is confusing" --run-dir .rs\autopilot\runs\starter-shop --format json
target\release\rs.exe autopilot decision .rs\autopilot\runs\starter-shop --decision "Use the shop direction" --constraint "Keep combat nonviolent" --format json
target\release\rs.exe autopilot align .rs\autopilot\runs\starter-shop --format json
target\release\rs.exe autopilot journal .rs\autopilot\runs\starter-shop --entry "Decision alignment passed and the run is ready for the next guardrail packet" --command "rs autopilot align .rs\autopilot\runs\starter-shop --format json" --result "aligned" --evidence alignment.json --format json
target\release\rs.exe autopilot opportunities --run-dir .rs\autopilot\runs\starter-shop --format json
target\release\rs.exe autopilot work-order --run-dir .rs\autopilot\runs\starter-shop --format json
target\release\rs.exe autopilot work-check .rs\autopilot\runs\starter-shop --format json
target\release\rs.exe autopilot cycle .rs\autopilot\runs\starter-shop --format json
target\release\rs.exe autopilot diagnose .rs\autopilot\runs\starter-shop --format json
target\release\rs.exe autopilot act .rs\autopilot\runs\starter-shop --dry-run --format json
target\release\rs.exe autopilot loop .rs\autopilot\runs\starter-shop --dry-run --format json
target\release\rs.exe smoke regression --studio "Demo Place" --out .rs\autopilot\runs\starter-shop\smoke.json --upload-mock
```

## Implementation Plan

### Phase 0: Schema And Artifact Foundation

- Add `docs/autopilot-spec.md`.
- Add Rust structs for `AutopilotPlan`, `AutopilotOperation`, `AutopilotRisk`, and `AutopilotRunArtifacts`.
- Add JSON schema-style validation in Rust.
- Add artifact writer under `.rs/autopilot/runs`.
- Add redaction helpers for secrets and local credentials.

Exit criteria:

- Invalid plans fail before any Studio call.
- Valid plans round-trip through JSON tests.
- Artifact folders are deterministic and safe to inspect.

### Phase 1: Deterministic Plan Runner

- Add `rs autopilot plan --from-manifest`.
- Add at least one built-in recipe: `starterShop`.
- Lower recipe output into the common plan schema.
- Implement `preview` as dry-run execution.
- Implement `apply` by delegating to existing commands and `/apply-plan`.
- Write `report.md`.

Exit criteria:

- Starter shop dry-run and apply work in a live Studio place.
- Re-running does not duplicate owned instances.
- Apply refuses without `--yes`.

### Phase 2: Validation And Rollback Hardening

- Create rollback package before mutation when scope exists.
- Integrate `validate` before and after apply.
- Integrate `diff` after apply.
- Integrate `history` snapshot restoration where available.
- Add rollback-on-error orchestration.

Exit criteria:

- Simulated mid-plan failure restores the target scope.
- Reports clearly distinguish applied, skipped, refused, and rolled-back operations.

### Phase 3: AI Planner Provider

- Add provider interface that returns strict plan JSON.
- Feed only sanitized bounded context into the provider.
- Add command flags for provider selection once configuration is designed.
- Reject unsupported or high-risk AI output before preview.
- Record model/provider metadata in `request.json` without secrets.

Exit criteria:

- Prompt-to-plan works for the starter shop demo.
- The same plan validation and approval gates apply to AI and non-AI plans.
- Refused operations are reported with actionable reasons.

### Phase 4: Studio Review Panel

- Add optional plugin toolbar button for Autopilot review.
- Let Studio users inspect pending operations, changed paths, risk, and validation results.
- Keep CLI approval as the source of truth initially.

Exit criteria:

- The panel can display the current pending plan and last run report.
- It does not apply changes independently of approval rules.

### Phase 5: Asset And Package Intelligence

- Teach plans to upload or update assets through explicit Open Cloud profiles.
- Support package updates with conflict policies.
- Surface asset moderation or permission blockers as validation failures.
- Add package-aware rollback and report links.

Exit criteria:

- Asset operations are fully reported and never success-shaped.
- Package updates can be previewed, applied, verified, and rolled back.

## MVP Demo Acceptance Criteria

The first public "wow" demo should satisfy all of these:

1. A fresh Studio place is open and registered with the bridge.
2. The user runs one high-level Autopilot command for a starter shop feature.
3. Dry-run writes `plan.json`, `preview.json`, and `report.md` without mutating Studio.
4. The plan shows created `ReplicatedStorage` remotes, server validation script, client UI script, and a simple `StarterGui` UI.
5. Applying with `--yes --validate --rollback-on-error` creates the feature.
6. Validation after apply has zero `fail` diagnostics for the touched scope.
7. Re-running the same plan does not duplicate instances.
8. A rollback artifact exists and is named in the report.
9. The final report lists every changed path.
10. No secrets, API keys, or private profile values appear in artifacts.

## Risks

- Planner output can look plausible while being structurally unsafe. Strict schema validation and operation allowlists are mandatory.
- Studio state can change between preview and apply. Plans need preconditions and should re-check them immediately before mutation.
- Script generation is high leverage but risky. MVP script writes should be small, explicit, and easy to inspect.
- Rollback cannot always restore external cloud asset side effects. Cloud operations must be clearly marked and separately reported.
- Large places can produce too much context. Context collection needs depth limits, filtering, and redaction.
- Force-updating non-owned instances can surprise creators. Keep force explicit and noisy.

## Open Questions

- Should the default scope be `game`, `Workspace`, or a required explicit path?
- Should Autopilot apply use `batch` internally, or call command modules directly?
- How should generated scripts be formatted and linted before insertion?
- Which provider configuration format should be used for AI planning?
- Should Studio review UI be required before public release, or remain a later polish layer?
- Should rollback use `history undo`, `package import`, or both depending on operation kind?

## Product Positioning

Autopilot should be described as:

> A safe automation layer for Roblox Studio that turns intent into reviewed, validated, rollbackable changes.

The feature is compelling because it combines three things that are rarely available together:

- Creator-friendly intent input.
- Real Studio mutation power.
- Engineering-grade safety, review, validation, and rollback.

That combination is the product's true wow factor.
