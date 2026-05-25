# AI Studio Autopilot Wow Factor Spec

## Product Thesis

The true wow factor for `roblox-studio-cli` is an AI build companion that can stay with a Roblox creator across the whole loop: understand a game idea, generate a safe offline candidate, explain what changed, present the best player moment, route creator feedback, remember taste and constraints, and launch a fresh AI model with the right context after compaction or handoff.

This feature is not "AI writes a script." It is a proof-bound collaboration system. The CLI gives the AI eyes, hands, judgment, memory, and stop rules so ambitious Studio work remains reviewable, reversible, and honest.

## North Star Demo

```powershell
rs autopilot start "make a cozy beginner tycoon with droppers, upgrades, coins, save data, and a bright shop"
rs autopilot wow-plan .rs\autopilot\runs\tycoon --root .rs\autopilot\runs
rs autopilot moment-sprint .rs\autopilot\runs\tycoon --root .rs\autopilot\runs
rs autopilot creator-demo .rs\autopilot\runs\tycoon --root .rs\autopilot\runs
rs autopilot demo-loop .rs\autopilot\runs\tycoon --message "Looks good but make the shop button brighter"
rs autopilot remember .rs\autopilot\runs\tycoon --root .rs\autopilot\runs
rs autopilot best-friend .rs\autopilot\runs\tycoon --root .rs\autopilot\runs
```

The creator sees a playable direction, a memorable demo moment, an evidence-backed status, and a clear approval boundary. A new AI sees one launch packet with project memory, creator preferences, current task context, safe claims, forbidden claims, and the first safe action.

## Users

- Roblox creators who want visible game progress without losing control of their Studio place.
- AI coding agents that need concrete commands, artifacts, and guardrails instead of vague memory.
- Technical collaborators who need auditability: what was generated, what was reviewed, what is safe to claim, and what still needs live proof.

## Goals

- Turn a vague prompt into a coherent Roblox gameplay slice and a reviewable offline run.
- Make the product hook explicit through a selected "wow moment," not only generic feature generation.
- Preserve creator feedback as reusable project memory.
- Package fresh-AI context so a model can resume without rereading the entire repo or hallucinating state.
- Keep every mutation behind explicit live gates, approval packets, proof artifacts, and rollback expectations.
- Let agents execute only safe offline Autopilot actions through internal handlers.

## Non-Goals

- Do not let an AI directly mutate Studio from a chat response.
- Do not treat a generated plan as proof that Studio was changed.
- Do not treat creator enthusiasm as live apply approval.
- Do not execute arbitrary Luau or unsupported planner output.
- Do not mark work complete without proof, acceptance, and completion-audit evidence.
- Do not require a live Studio session for the offline wow loop.

## Current Foundation

The repository already has the key command ladder needed for this product direction:

- `start`, `drive`, `review-pack`, `approval`, `proof`, `acceptance`, `privacy`, and `completion-audit` for safe offline run creation and claim discipline.
- `model-pack`, `task-pack`, and `best-friend` for fresh-AI context packaging.
- `wow-plan`, `moment-pack`, `moment-sprint`, `moment-decision`, and `creator-demo` for selecting, building, comparing, and presenting a memorable player moment.
- `demo-response`, `demo-loop`, `demo-check`, `demo-reply`, `demo-learn`, and `remember` for post-demo feedback routing and durable learning.
- `self-check`, `act`, `cycle`, `diagnose`, `work-order`, and `work-check` for bounded safe offline progress and pre-action claim discipline.
- `ready`, `live-gate`, `apply`, `certify`, `rollback`, and `health` for the later live mutation boundary.

This spec focuses on making those pieces feel like one product experience.

## Core Experience

### 1. Understand The Creator

The AI starts with `intake`, `pitch`, `storyboard`, or `start` to interpret the prompt, infer recipes, record assumptions, and create a safe offline run. The creator should get options and a clear next command when the request is broad.

Expected outputs:

- `intake.json` and `intake.md`
- `proposal.json` and `storyboard.json` when proposal flow is used
- `start.json`
- `review-pack.json`
- `approval.json`
- `privacy.json`

### 2. Build A Reviewable Slice

The run produces generated Luau, a strict plan, preview evidence, source audit, bundle state, gameplay critique, proof, and acceptance. The system must keep saying "offline candidate" until live apply and playtest proof exist.

Expected outputs:

- `plan.json`
- `preview.json`
- `generated/**/*.lua`
- `source-audit.json`
- `bundle.json`
- `certification.json`
- `gameplay-critique.json`
- `proof.json`
- `acceptance.json`
- `completion-audit.json`

### 3. Find The Product Hook

`wow-plan` reads the run and ranks player-facing moments. A good wow moment is concrete, demoable, safe to build offline, and tied to the creator's prompt.

Examples:

- "First Upgrade Ceremony" for a tycoon: the first purchase triggers visual feedback, UI copy, and a clearer economy beat.
- "Quest Turn-In Moment" for an adventure hub: the player gets a visible reward, next objective, and celebration.
- "Shop Reveal" for a simulator: a clearer button, previewed item cards, and first purchase guidance.

Expected outputs:

- `wow-plan.json`
- `wow-plan.md`
- selected idea
- safe next command
- proof needs
- forbidden claims

### 4. Generate The Wow Candidate

`moment-pack` turns the selected idea into a task packet. `moment-sprint` executes the supported offline candidate path. The source run stays intact; the candidate run is a separate reviewed continuation.

Expected outputs:

- `moment-pack.json`
- `moment-sprint.json`
- candidate run folder
- candidate `review-pack.json`
- candidate `showcase.json`
- candidate `proof.json`
- candidate `completion-audit.json`

### 5. Choose What To Show

`moment-decision` compares source and candidate evidence, refreshes claim checks, and recommends what the AI should present. It must not record approval or imply the candidate is live.

Expected outputs:

- `moment-decision.json`
- `comparison.json`
- recommended run path
- safe claims
- blockers and warnings

### 6. Demo The Moment

`creator-demo` writes the presentation packet: short talk track, proof table, what is ready for review, what is not proven, and which approval boundary comes next. This is the emotional product moment.

Expected outputs:

- `creator-demo.json`
- `creator-demo.md`
- proof-bound talk track
- approval boundary
- next actions

### 7. Learn From The Creator

After the creator reacts, `demo-response` classifies the message, `demo-loop` turns it into a safe handoff, `demo-check` verifies follow-through, `demo-reply` writes checked wording, `demo-learn` extracts reusable signals, and `remember` consolidates those signals into durable context.

Feedback examples:

- "Make the shop button brighter" becomes a feedback patch route.
- "Looks good, go ahead" becomes approval preparation, not live apply.
- "Actually make it a space tycoon" becomes redirection and decision memory.

Expected outputs:

- `demo-response.json`
- `demo-loop.json`
- `demo-check.json`
- `demo-reply.json`
- `demo-learn.json`
- `remember.json`
- refreshed `project-memory.json`
- refreshed `creator-preferences.json`
- refreshed `game-bible.json`
- refreshed `ai-playbook.json`

### 8. Launch The Next AI Best Friend

`best-friend` is the handoff climax. It refreshes memory and task context, then writes a single launch packet that tells a fresh AI who the creator is, what the game is becoming, what evidence exists, what to read first, what to say, what not to say, and what action is safe.

Expected outputs:

- `best-friend.json`
- `best-friend.md`
- opening prompt
- companion contract
- context cards
- required read order
- first safe action
- safe claims
- forbidden claims
- blockers and warnings
- source artifact links

## `best-friend` Contract

`rs autopilot best-friend <run-dir>` must be non-mutating. It can refresh offline context artifacts, but it must not execute the task, approve live apply, mutate Studio, prove live playtests, publish, or mark work complete.

The packet should answer these questions:

1. What is this project and current run?
2. What does the creator appear to value?
3. What did the last demo or feedback teach us?
4. What evidence is trustworthy?
5. What is safe to claim?
6. What must not be claimed?
7. What should the next AI read first?
8. What is the first safe action?
9. Where are the source artifacts?
10. What would require creator approval or live Studio readiness?

## Artifact Schema Expectations

All major JSON artifacts should include:

- `schemaVersion`
- `status` or `verdict`
- source `runDir` and relevant root paths
- generated or refreshed artifact paths
- blockers
- warnings
- safe claims
- forbidden claims
- next actions

`best-friend.json` should additionally include:

- `openingPrompt`
- `companionContract`
- `contextCards`
- `requiredReadOrder`
- `firstSafeAction`
- `agentBrief`
- `sourceLinks`

`contextCards` should be short, typed summaries. Recommended card kinds:

- `projectMemory`
- `creatorPreferences`
- `gameBible`
- `aiPlaybook`
- `runState`
- `task`
- `proof`
- `safety`

## Safety Model

The feature is allowed to refresh offline artifacts. It is not allowed to cross live boundaries.

Safe offline actions include:

- checking proposed AI wording and commands with `self-check`
- generating or refreshing Markdown and JSON run packets
- creating separate offline candidate runs
- reading existing run artifacts
- checking expected artifact presence
- writing claim-check and response drafts
- routing feedback into planner packets or decisions

Gated actions include:

- Studio apply
- live readiness assertions
- Open Cloud upload or publish
- live playtest proof
- approval recording that implies mutation permission
- package import into Studio

Every creator-facing message must pass through claim boundaries. The AI should prefer "ready for review," "offline candidate exists," or "approval packet is prepared" over "done" unless completion evidence exists.

## Studio Panel Experience

The CLI remains the source of truth. The Studio panel should make the current review packet visible where creators work.

The panel should show:

- current run name and status
- selected wow moment
- proof table
- changed paths or planned changed paths
- blockers and warnings
- safe-to-say summary
- do-not-say guardrails
- approval boundary
- rollback readiness
- latest creator feedback route

The panel should not be the first implementation of approval unless the CLI approval contract is preserved.

## Implementation Plan

### Phase 1: Spec And Command Cohesion

- Keep `docs/autopilot-spec.md` aligned with implemented commands.
- Keep `README.md` command examples concise.
- Ensure `best-friend`, `remember`, `demo-*`, and `moment-*` docs all state non-mutating boundaries.
- Ensure `act` only dispatches supported safe offline handlers.

Exit criteria:

- Command docs match CLI variants.
- Each command has JSON and Markdown artifact expectations.
- No doc says live work happened when only offline artifacts exist.

### Phase 2: Best-Friend Launch Packet

- Implement or preserve `best-friend` as the one-shot fresh-AI entry point.
- Refresh `remember`, `model-pack`, `task-pack`, `opportunities`, and `work-order` before writing the packet.
- Generate context cards from durable memory and task evidence.
- Pick one first safe action from evidence, not prose guessing.
- Preflight the launch claim and first safe action with `self-check`.
- Include a checked opening reply the AI can send before acting.
- Prefer an `act`-wrapped first command when the first action has a safe internal handler.
- Provide `first-turn` as the one-step execution receipt for that protected command.
- Provide `best-friend-loop` for bounded non-repeating protected turns.
- Provide `best-friend-reply` for checked creator-facing wording after protected turns.
- Provide `best-friend-turn` as the one-command "listen, safely act, reply honestly" receipt.
- Provide `best-friend-session` as the first-contact command that bootstraps or resumes before the turn.
- Provide `best-friend-check`, `best-friend-rescue`, and `best-friend-mentor` so a fresh model can audit launch readiness, recover from blockers, and understand why the next move is safe before acting.
- Provide `best-friend-pilot` as the one-move co-pilot receipt that mentors first, then either runs one protected offline action and checked reply or prepares rescue.
- Surface `best-friend-pilot` in the Studio review panel through `publish-review` so the creator can inspect co-pilot status, selected protected command, and checked companion message.
- Provide `best-friend-control` as the first-read operator receipt that decides whether the AI should pilot, rescue, publish the co-pilot receipt, refresh the checked reply, or speak.
- Provide `best-friend-operate` as the one-step operator that executes only the selected offline branch, refreshes control, and stops before Studio publish or a second move.
- Provide `best-friend-runner` as the bounded supervisor that repeats safe operator steps until checked speech, Studio publish, blocker, repeat, or step-limit boundaries.
- Surface `best-friend-runner` in the Studio review panel through `publish-review` so the creator can inspect bounded supervisor status, stop reason, step count, and checked message.
- Include explicit forbidden claims.

Exit criteria:

- A fresh AI can read `best-friend.md` and know exactly what to do next.
- The packet includes `best-friend-self-check.json` so the first action and opening claim are already guarded.
- The packet includes an opening reply that names the checked offline move and live-work boundary.
- The first recommended executable command goes through `rs autopilot act` when supported.
- `first-turn.json` proves whether the protected first action was selected, dry-run, executed, blocked, or failed.
- `best-friend-loop.json` proves how many protected turns ran and why the loop stopped.
- `best-friend-reply.json` includes a self-checked message before the AI speaks to the creator.
- `best-friend-turn.json` ties chat routing, protected work, and checked wording into one audited AI turn.
- `best-friend-session.json` ties first-contact bootstrap/resume, protected turn, and checked wording into one audited companion surface.
- `best-friend-mentor.json` turns launch control into read order, coaching cards, mistake traps, and one safe next action or rescue command.
- `best-friend-pilot.json` proves whether the companion coached itself, executed one protected offline move, prepared rescue, and produced a checked message.
- `studio-review.json` carries the co-pilot receipt into the Studio panel when `best-friend-pilot.json` exists or is passed explicitly.
- `studio-review.json` carries the runner receipt into the Studio panel when `best-friend-runner.json` exists or is passed explicitly.
- `best-friend-control.json` selects the next branch from launch-control, pilot, and Studio-review evidence before the AI acts or speaks.
- `best-friend-operate.json` proves whether the selected branch executed, what changed, and what control says next.
- `best-friend-runner.json` proves every bounded operator step and why the AI stopped before live work or repetition.
- The packet points back to source artifacts instead of becoming the only source of truth.
- Tests cover direct command output and `act` dispatch.

### Phase 3: Wow Moment Loop

- Rank player-facing moments from run artifacts.
- Generate a separate offline candidate run for the selected moment.
- Compare candidate and source evidence.
- Produce a proof-bound creator demo packet.
- Provide `wow-session` as the first-contact/resume command that prepares the proof-bound wow demo without manual command stitching.

Exit criteria:

- The creator can see why this candidate is more exciting than the baseline.
- The AI can explain what is offline, what is proven, and what still needs live approval.
- `wow-session.json` ties companion bootstrap/resume, selected wow idea, recommended run, demo packet, and claim guardrails into one audited creator-review surface.

### Phase 4: Post-Demo Learning

- Route creator reactions into feedback, approval prep, redirection, or checked response.
- Verify follow-through before replying.
- Distill reusable preference and constraint signals.
- Refresh durable memory and playbook context.
- Provide `demo-session` as the one-command post-demo reaction handler that routes, checks, replies, learns, and remembers.
- Provide `best-friend-arc` as the top-level command that prepares the proof-bound demo and, when a reaction is present, returns the checked post-demo reply plus remembered feedback.

Exit criteria:

- The next run reflects creator feedback without requiring the AI to remember chat history.
- Approval-like language never becomes automatic live mutation.
- `demo-session.json` returns checked reply wording only after route-specific evidence and refreshed memory artifacts are written.
- `best-friend-arc.json` is the read-first receipt for the whole creator arc: demo-ready before the reaction, reply-ready after the reaction.

### Phase 5: Studio Review Polish

- Publish the review packet into the Studio panel.
- Make selected wow moment, proof, blockers, and approval boundary visible.
- Include `best-friend-arc` status, selected moment, checked demo message, checked post-demo reply, and next action in the panel payload.
- Include `best-friend` launch context so a creator or resumed AI can see the read order, checked opening reply, first safe action, protected `act` wrapper, and stop boundaries inside Studio.
- Keep CLI as authority for mutation and proof.

Exit criteria:

- The creator can review the AI's recommendation inside Studio before approving any live work.
- A published Studio panel can show whether the arc is demo-ready or reply-ready without treating that review as approval.
- The panel can display best-friend context without executing the first action or treating the handoff as creator approval.

## Validation Plan

Run these checks before claiming the feature is ready:

```powershell
cargo fmt --check -p rs
cargo test -p rs best_friend
cargo test -p rs self_check
cargo test -p rs task_pack
cargo test -p rs remember
cargo test -p rs act_
cargo test -p rs
cargo build --release -p rs
```

Run a release smoke on a clean target folder:

```powershell
$root = Join-Path 'target' 'autopilot-best-friend-smoke'
$run = Join-Path $root 'tycoon'
Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue
.\target\release\rs.exe autopilot start 'make a tycoon with shop and saves' --root $root --run-dir $run --no-markdown --format json
.\target\release\rs.exe autopilot demo-loop $run --message 'Looks good but make the shop button brighter' --root $root --no-markdown --format json
.\target\release\rs.exe autopilot remember $run --root $root --no-markdown --format json
.\target\release\rs.exe autopilot best-friend $run --root $root --opportunity 'Record AI work journal' --no-markdown --format json
```

Minimum smoke assertions:

- `best-friend.json` exists.
- `best-friend-self-check.json` exists and reports `readyToProceed`.
- `remember.json` exists.
- `project-memory.json`, `creator-preferences.json`, `game-bible.json`, and `ai-playbook.json` exist under the root context.
- `best-friend.json` has context cards, required read order, first safe action, safe claims, and forbidden claims.
- Creator feedback appears in refreshed preference or memory artifacts when routed through `demo-loop` and `remember`.

## Success Metrics

- A new AI can resume from `best-friend.md` without asking the creator what happened.
- Creator-facing replies avoid unsupported completion claims.
- Offline wow candidates are generated in separate run folders.
- Feedback is preserved as durable preference, canon, or playbook context.
- The first safe action is executable through a supported offline handler or explicitly gated.
- Live Studio mutation remains impossible without the existing approval and readiness path.

## Risks

- The artifact chain can become too large for a model; `best-friend` must summarize and link rather than embed everything.
- Generic recipes can make wow moments feel samey; moment selection should reward prompt-specific player experience.
- The AI may overclaim after a good demo packet; safe and forbidden claims must stay prominent.
- Creator approval wording is ambiguous; approval-like chat must still route to readiness and live gates.
- Separate candidate runs can confuse agents; source and recommended run paths must be explicit.

## Open Questions

- Should `remember --best-friend` become the default, or stay opt-in to keep memory consolidation lighter?
- Should the Studio panel make `best-friend` context the default after `remember`, or keep it behind explicit `publish-review --best-friend` publishing?
- Should post-demo learning be project-wide by default, or require a run-specific opt-in?
- Should `moment-sprint` support multiple candidate ideas in one command, or keep exactly one selected moment per sprint?

## Definition Of Done

The feature earns the wow label when a creator can ask for a substantial Roblox slice, receive a reviewable offline candidate, see a memorable player moment, react naturally, have that reaction safely routed into the next work packet, and hand the project to a fresh AI that knows the game, the creator's taste, the proof state, the approval boundary, and the next safe action.
