# AI Best Friend Roadmap

## Framing Question

If I were an AI model using `rs` to help a user build Roblox games, what features would I need and why?

I would need the CLI to be a reliable sensory system, a safe set of hands, and a memory trail. Roblox Studio is visual, stateful, and easy to mutate accidentally; an AI agent needs tools that turn that environment into bounded facts, reversible plans, and repeatable workflows.

## 1. Fast, Sanitized Context Bundles

An AI needs a concise snapshot of the open Studio place: services, scripts, remotes, assets, packages, duplicate names, validation failures, and dependency edges. It should not need to scrape huge trees or risk leaking secrets.

Implemented first: `rs autopilot context` writes redacted context bundles with snapshot, validation, dependency graph, optional bounded read data, supported operation kinds, and the deterministic recipe catalog.

Why: Most AI mistakes come from acting without enough current state.

## 2. Structured Plans, Not Raw Code Execution

An AI should produce a JSON plan with typed operations instead of arbitrary Luau. The CLI should validate that plan, show a preview, and refuse unsupported or unsafe operations.

Implemented first: `rs autopilot plan`, `preview`, `apply`, `run`, and `report` with deterministic recipes and schema validation.

Why: Plans are reviewable, reproducible, and testable; raw generated scripts are not.

## 3. Safe Mutation Primitives

An AI needs to create instances, upsert scripts, import assets, validate, snapshot, repair, and package through first-class commands. Every mutation should require approval and produce changed paths.

Implemented next: Autopilot plans now lower dependency checks, publish checks, local image imports, local mesh imports, already-uploaded asset imports, Open Cloud uploads, package imports, and smoke suites. `preview --live` checks Studio preconditions before dry-running supported operations.

Why: The model should compose reliable tools, not reinvent plugin behavior.

## 4. Rollback And History Everywhere

An AI must be able to say, "I can undo this." Before applying a feature plan, the CLI should capture rollback artifacts and record a report.

Implemented next: conservative automatic restore from Autopilot rollback packages for child scopes such as `Workspace.Shop`. `deleteInstance` is now supported only for danger-level destructive plans, refuses live apply without `--force --rollback-on-error`, and lowers through the same ownership-checked Studio fix-plan path. Whole-place and service-level restores still require manual review or a future history-backed restore path.

Why: Ambitious AI changes become usable only when they are reversible.

## 5. Game Feature Recipes

An AI benefits from canonical recipes for common Roblox systems: starter shop, collectible coins, quests, round manager, lobby teleport, obby checkpoints, tycoon bases, tools, enemy encounters, NPC interaction, inventory, save data scaffolding, and admin panels.

Implemented first: deterministic `starterShop`, `collectibleCoin`, `questSystem`, `roundManager`, `inventorySystem`, `adminPanel`, `lobbyTeleport`, `obbyCheckpoint`, `tycoonCore`, `toolSystem`, `enemyEncounter`, `npcInteraction`, and `saveDataScaffold` recipes.

Why: Recipes give the model safe defaults and let users get visible game features quickly.

## 6. Feature Stack Composition

An AI should be able to assemble a coherent starter game loop in one reviewable plan, not stitch separate features manually across multiple runs.

Implemented next: `rs autopilot compose` accepts presets, repeated recipes, prompt inference, or a composition manifest such as `examples/full-starter-game.autopilot-compose.json`. It namespaces generated files by recipe, merges preconditions, deduplicates verifier operations, and appends one final validation/publish/smoke sequence. Genre presets now include a `tycoonPrototype` stack for a tycoon base plus shop, save-data, and admin support. `rs autopilot tune` now turns creator intent into an explicit compose manifest with per-recipe names, currency labels, shop items, coin values, tycoon drop values, and upgrade prices, and `compose --from-manifest` honors those recipe objects.

Why: The "wow" moment is not a single helper script; it is a full Roblox gameplay slice that remains previewable, applyable, reportable, and rollbackable.

## 7. Verification Loops

An AI needs post-change proof: validation summaries, dependency reports, smoke tests, diffs, and publish checks.

Implemented next: generated game-feature plans include `validate`, `deps`, and `publishCheck` verifier operations, and `rs autopilot apply --smoke <suite>` now runs existing smoke suites and records PASS/FAIL evidence in the run report. `rs autopilot health` writes the applied-run health gate and only returns `appliedAndHealthy` when apply, post-apply validation, mandatory `regression` smoke, rollback artifact, live playtest proof, and proof ledger all pass. `preview.json` now carries an integrity seal for the reviewed plan and generated scripts, and `apply` refuses when those files drift after preview. `rs autopilot audit-sources` scans generated Luau for unsafe dynamic execution, external numeric requires, direct HTTP usage, debug/environment introspection, missing files, unsafe paths, and secret markers; `kickoff` writes source-audit artifacts before bundling, and `certify` includes the source-audit gate.

Why: The useful answer is not "I changed it"; it is "I changed it and here is why it is still healthy."

## 8. Agent-Friendly Output

Every command should have JSON output, stable machine-readable error codes, exact changed paths, and short human summaries.

Implemented next: `rs autopilot recipes --format json` exposes the deterministic feature catalog with aliases, prompt hints, preconditions, created paths, generated files, and risk. `rs autopilot explain --format json` turns any plan into an agent-readable review packet with validation status, blockers, warnings, generated files, operation groups, and recommended next commands. `rs autopilot coach --format json` reads a run folder and returns the next safe agent actions, live-Studio requirements, mutation risk, blockers, and missing artifacts. `rs autopilot bundle --format json` writes a hashed run handoff manifest so another agent can audit artifacts and resume safely, and `rs autopilot verify-bundle --format json` proves that manifest still matches local artifacts before a handoff is trusted. `rs autopilot handoff --format json` creates the one-file continuity packet an AI should ingest before continuing: bundle status, blockers, artifacts, agent brief, and exact next commands, with a companion Markdown summary for humans. `rs autopilot runs --format json` indexes prior run folders so an agent can pick the right continuation point without guessing from directory names. `rs autopilot mission --format json` creates a project-level AI mission packet that combines recent runs, an active continuation point, prompt-to-recipe recommendations, blockers, and safe next commands. `rs autopilot memory --format json` creates a compact project-memory ledger with active run, known created Studio paths, generated files, inferred recipes, gameplay critique verdicts/gaps, certification state, blockers, warnings, and next actions. `rs autopilot critique --format json` scores a planned gameplay slice for core loop, interaction, rewards, progression, UI feedback, onboarding, server authority, verification, and continuity. `rs autopilot playtest --format json` writes recipe-aware live playtest scenarios with exact steps and expected evidence for the generated slice. `rs autopilot architect --format json` turns a vague creator request into a staged build architecture with genre intent, player promise, recipe stack, phase commands, verification gates, live-readiness requirements, and rollback/apply path. `rs autopilot kickoff --format json` runs the full offline startup sequence and writes architecture, tuned compose manifest, compose, preview, handoff, bundle, source audit, certification, gameplay critique, playtest plan, project memory, and generated-source artifacts in one ready-to-review run folder. `rs autopilot audit-sources --format json` gives agents a machine-readable generated-Luau safety report. `rs autopilot planner-pack --format json` writes a provider-ready, redacted planning packet with the strict plan contract, supported operation kinds, recipe catalog, artifact context, safety rules, and expected JSON-only model output. `rs autopilot adopt-plan --format json` turns strict model output back into a normal certified run folder, rejecting unsupported fields and unsafe source paths before writing preview, source-audit, handoff, bundle, planner-pack, and certification artifacts. `rs autopilot certify --format json` creates a deterministic go/no-go certificate for a run, including plan validity, generated-file presence, source audit, preview safety, preview integrity, bundle verification, handoff state, apply status, and live-readiness requirements. `rs autopilot ready --format json` gives an AI a bounded readiness gate for bridge, Studio selector, protocol, and required plugin capabilities. `rs doctor --format json` provides machine-readable live-readiness blockers and fix commands. Broader command-envelope standardization remains a cross-command hardening item.

Why: AI agents need stable contracts, not prose-only terminal output.

## 9. Prompt-To-Architecture Planning

An AI should not have to jump from a one-sentence user wish straight into mutation. It needs a build architecture that explains the intended game loop, recommended recipe stack, phase order, commands, validation gates, and exact point where live Studio approval becomes necessary.

Implemented next: `rs autopilot architect` writes `architect.json` and `architect.md` without contacting Studio. The report maps prompts such as tycoon, adventure hub, obby, combat, quests, and full starter-game requests into a staged path: tune, compose from the tuned manifest, explain, handoff, ready, live preview, apply with rollback, and certify. `rs autopilot kickoff` automates the offline part of that path, producing a ready-to-review run packet with architecture, `tune.autopilot-compose.json`, `tune.md`, plan, preview, handoff, bundle, certification, gameplay critique, planner pack, generated scripts, and a `kickoff.json` summary.

Why: The most useful AI partner is not just fast; it is legible, staged, and hard to accidentally misuse.

## 10. Provider-Ready Planning Packets

An external AI planner should get a sealed brief, not raw power. It needs redacted context, known-safe operations, examples of the exact plan schema, and instructions that forbid Markdown wrappers, unsupported operations, secret leakage, arbitrary Luau execution, and fake live-success claims.

Implemented next: `rs autopilot planner-pack` writes `planner-pack.json` and `planner-pack.md` from either a prompt plus context file or an existing run directory. The packet includes a model task, plan contract, safety rules, agent instructions, operation catalog, deterministic recipes, expected strict JSON output, redacted context, run artifact context, and validation commands.

Why: This is the bridge from "AI can suggest code" to "AI can safely collaborate with this CLI."

## 11. Strict Model Output Intake

An AI needs a safe way to hand work back to the CLI. A returned plan should not be trusted just because it is JSON; it needs strict field checks, generated-source materialization, source auditing, preview sealing, bundle hashing, handoff packets, and certification before any live Studio mutation.

Implemented next: `rs autopilot adopt-plan` accepts a strict AI-generated `rs.autopilot.plan.v1` file plus a `generated/` source root, rejects wrappers and unsupported fields, copies referenced scripts into a run folder, and produces the same preview, source-audit, handoff, bundle, planner-pack, certification, and `adopt-plan.json` artifacts as native runs.

Why: This closes the loop between model planning and production-safe execution.

## 12. Project Memory Ledger

An AI should be able to resume after context loss, hand off to another agent, or compare a new request against what already exists. It needs a compact memory of known created Studio paths, generated files, recipes, active run state, certification status, blockers, warnings, and safe next actions.

Implemented next: `rs autopilot memory` writes `project-memory.json` and `project-memory.md` from prior run folders. The report summarizes active continuation state plus per-run operation counts, created paths, generated files, inferred recipes, gameplay critique verdicts/gaps, certification flags, and next commands.

Implemented next: `rs autopilot control` writes `control.json` and `control.md` as the AI session entry packet. It combines memory, next, roadmap, judgment, and repair-plan evidence, then returns one recommendation, a command queue, artifact paths, blockers, warnings, and do-not-claim guardrails.

Implemented next: `rs autopilot brief` writes `user-brief.json` and `user-brief.md` as the user-facing claim guard. It turns the current control state into one concise update, safe-to-say claims, do-not-say claims, evidence, blockers, warnings, and next actions so an AI can report progress without hallucinating live proof.

Implemented next: `rs autopilot intake` writes `intake.json` and `intake.md` as the request-understanding gate. It interprets a creator prompt into intent, confidence, recipe stack, assumptions, clarifying questions, acceptance criteria, continuity warnings, and first commands so an AI can ask only the questions that matter and otherwise proceed with explicit assumptions.

Implemented next: `rs autopilot start` writes `start.json` and `start.md` as the one-command offline session bootstrap. It always writes intake, pauses when clarification is truly blocking, or with safe prompts creates kickoff, control, user-brief, review-pack, evidence-kit, capsule, approval, proof, acceptance, and privacy artifacts so an AI begins with build evidence, claim discipline, creator approval context, and a handoff-ready review surface.

Implemented next: `rs autopilot cockpit` writes `cockpit.json` and `cockpit.md` as the AI session dashboard. It refreshes project memory, next move, roadmap, mission control, user brief, proof, acceptance, and privacy artifacts in one offline pass, then exposes readiness flags, panels, claim guardrails, evidence, trusted artifacts, and an exact command queue.

Implemented next: `rs autopilot capsule` writes `agent-capsule.json` and `agent-capsule.md` as the AI-to-AI continuation packet. It refreshes the cockpit, lists required context artifacts, generates a copy/paste resume prompt, carries safe-to-say and do-not-say claims forward, and blocks clean handoff when privacy proof is not passing.

Implemented next: `rs autopilot review-pack` writes `review-pack.json` and `review-pack.md` as the creator/AI review packet. It refreshes approval, proof, acceptance, privacy, evidence, and capsule artifacts into one decision surface with safe claims, forbidden claims, decision gates, artifact links, and exact next commands.

Implemented next: `rs autopilot proof` writes `proof.json` and `proof.md` as the claim-evidence ledger. It maps each success claim to plan, source audit, preview, bundle, handoff, certification, gameplay, playtest, apply, rollback, and readiness proof, then records `claimReady`, `doNotClaim`, missing proof, and exact next commands.

Implemented next: `rs autopilot acceptance` writes `acceptance.json` and `acceptance.md` as the creator-intent scorecard. It checks prompt-to-recipe fit, generated files, preview/source-audit/bundle/handoff state, gameplay verdict, playtest checklist, and live proof, then separates `offlineSatisfied` from `finalSatisfied`.

Implemented next: `rs autopilot rollback` writes `rollback.json` and `rollback.md` as the undo-readiness guide. It reads `apply.json`, verifies the rollback package exists, computes whether the scope has a safe restore parent, and emits dry-run plus approved `rs package import` commands only when the artifact and scope make that honest.

Implemented next: `rs autopilot approval` writes `approval.json` and `approval.md` as the creator-consent packet. It reads the plan, preview, and certification evidence, then produces the exact approval prompt, live-readiness command, approved apply command, touched paths, generated files, blockers, warnings, and do-not-claim lines an AI should use before crossing the mutation boundary.

Implemented next: `rs autopilot privacy` writes `privacy.json` and `privacy.md` as the artifact handoff safety scan. It checks run JSON, Markdown, generated Lua, and companion text for unredacted secret-like fields, credential assignments, and private token shapes, redacts its own evidence, and blocks handoff claims until findings are fixed.

Implemented next: `rs autopilot next` writes `next.json` and `next.md` with one recommended next move for an AI agent. It reads a selected run or project root, weighs bundle, certification, gameplay critique, and memory evidence, then chooses whether to kickoff, critique, iterate, certify, refresh handoff, check live readiness, or inspect an applied report.

Implemented next: `rs autopilot roadmap` writes `roadmap.json` and `roadmap.md` with a short execution backlog. It groups evidence-backed commands into milestones such as stabilize, improve, build, live proof, and continuity, and records expected artifacts for each step.

Implemented next: `rs autopilot judge` writes `judgment.json` and `judgment.md` with a conservative readiness verdict for a run. It combines certification, gameplay critique, bundle verification, playtest-plan presence, and required artifacts, and it refuses to call a run production-ready without live apply and playtest evidence.

Implemented next: `rs autopilot record-playtest` writes `playtest-result.json` and `playtest-result.md` from live Studio observations. A passing result needs explicit evidence and an applied certification record; `judge` then consumes the result but still requires rollback proof and a verified bundle before marking a run production-ready.

Implemented next: `rs autopilot evidence` writes `evidence-kit.json` and `evidence-kit.md` as the live-proof collection kit. It refreshes playtest scenarios, creates an evidence folder layout for screenshots/logs/notes/exports, maps every expected observation to a suggested path, and emits exact `record-playtest` commands without claiming success.

Implemented next: `rs autopilot repair-plan` writes `repair-plan.json` and `repair-plan.md` from non-passing playtest evidence. It infers repair incidents from failed scenarios, notes, and evidence, maps them back to likely recipes or custom-plan needs, and emits planner/adopt/certify commands so an AI can turn live failure into the next safe patch.

Why: Durable memory turns Autopilot from a one-shot feature generator into a long-running build partner.

## 13. Gameplay Critique Loop

An AI should not stop at "the plan validates." It should ask whether the run actually forms a playable Roblox slice: clear interaction, repeatable loop, rewards, progression, feedback UI, onboarding, server authority, verification, and handoff continuity.

Implemented next: `rs autopilot critique` writes `gameplay-critique.json` and `gameplay-critique.md` for a run or plan. It assigns a verdict such as `needsGameplayLoop`, `needsPolish`, or `playableSlice`, records design gaps, and recommends exact recipe/command patches. `rs autopilot playtest` writes `playtest-plan.json` and `playtest-plan.md` with recipe-aware live playtest scenarios and expected evidence. `rs autopilot improve` closes the loop by selecting missing deterministic recipes from critique gaps and writing a fresh patch run with audit, handoff, bundle, certification, planner pack, critique, and playtest artifacts. `kickoff` now writes gameplay critique and playtest artifacts by default.

Why: The best AI partner catches "technically generated but not fun yet" before the user spends a live Studio apply on it.

## 14. Autonomous Improvement Loop

An AI should be able to act on its own critique without copying suggested commands out of prose. The CLI should transform "this slice lacks a repeatable loop" into a new reviewable patch run, with the same safety gates as any other Autopilot plan.

Implemented next: `rs autopilot improve --run-dir <run>` re-critiques the source run, filters out recipes already present, chooses the highest-priority blocker or warning patches, and writes `improve.json` plus `improve.md`. The patch run includes generated source, preview, source audit, handoff, bundle, certification, planner pack, gameplay critique, and playtest plan. Agents can force recipes with `--recipe` or bound ambition with `--max-recipes`.

Implemented next: `rs autopilot compare --base-run <run> --candidate-run <run>` writes `comparison.json` and `comparison.md` with score delta, verdict delta, recipe/path/generated-file differences, bundle state, certification verdicts, blockers, warnings, and next actions. This lets an AI judge whether an improved run is actually the better continuation before asking for live apply.

Implemented next: `rs autopilot iterate --run-dir <run>` automates the offline loop: critique the current run, create a deterministic patch candidate, write the full artifact packet, compare it to the current best, accept only better candidates, and stop at `playableSlice` or `needsCustomPlan`. The session writes `iteration.json` and `iteration.md` plus per-step candidate folders with their own comparison and playtest artifacts.

Implemented next: `rs autopilot sequence --run-dir <baseline> --run-dir <patch>` writes an ordered multi-run apply packet. It preserves the baseline-to-patch order, aggregates recipes and changed paths, downgrades weaker early gameplay verdicts to review warnings, verifies bundle state, and records the exact readiness, verify, and apply commands an agent must follow.

Why: This makes Autopilot feel less like a command catalog and more like a collaborator that can notice a weak loop, propose the next playable increment, and produce the artifacts needed to review it.

## 15. Studio Review Surface

The CLI is the source of truth, but an in-Studio review panel would make plans feel tangible: pending operations, risk, changed paths, validation state, and rollback status.

Implemented next: a plugin toolbar review panel that receives the latest live Autopilot preview/apply summary through the bridge and displays risk, operation counts, warnings, changed paths, rollback state, and report paths.

Implemented next: `rs autopilot publish-review` writes `studio-review.json` and `studio-review.md`, merges a run's plan/preview/apply/review/proof artifacts with optional `companion.json`, and publishes that non-mutating packet into the Studio review panel. The panel now shows AI handoff status, setup readiness, recommended candidate, blockers, safe claims, forbidden claims, and next actions before live preview or apply.

Why: Creators live in Studio; the safest approval moment is often visual.

## 16. Place Survey Before Planning

An AI should not design against a blank mental model when a place already has scripts, remotes, UI, assets, and hand-authored content. It needs a concise survey that says what exists, what is risky, where new work should go, and which recipes fit the current gaps.

Implemented next: `rs autopilot survey` writes `survey.json` and `survey.md` from either live Studio inspection or a saved `context.json`. It summarizes instance counts, validation state, system signals, ownership risks, duplicate names, asset references, safe mutation zones, suggested recipes, and planner/start next commands.

Why: This closes the loop between "inspect a place" and "propose a feature" so the AI can ground its next plan in the actual Roblox project.

## 17. Scout The Next Move

After a place survey, an AI still needs to fuse that evidence with the user's current request. It should choose a scope, decide which deterministic recipes fit, carry forward safe zones and warnings, and emit the exact next command instead of narrating possibilities.

Implemented next: `rs autopilot scout` writes `scout.json` and `scout.md` from a creator prompt plus `survey.json` or `context.json`. It merges intake, survey signals, suggested recipes, blockers, warnings, and do-not-do guardrails into one next-build packet with commands for `start`, `compose`, and `planner-pack`.

Why: This makes the CLI feel like an AI collaborator that can inspect, understand, and choose the next safe build move before generating code.

## 18. Session Work Order

Once an AI knows the next move, it needs a work-order artifact that converts that decision into the complete offline run surface. The packet should carry the scout decision, selected recipes, review artifacts, approval boundary, evidence kit, and do-not-do rules in one place.

Implemented next: `rs autopilot session` writes `session.json` and `session.md` from a scout, survey, context, or prompt. When the scout is ready, it bootstraps `start.json` inside the run folder and refreshes review-pack, evidence-kit, capsule, approval, proof, acceptance, privacy, control, and user-brief artifacts.

Why: This is the bridge from "I know what to build" to "I have the exact offline work packet needed to ask for approval and proceed safely."

## 19. Live Gate Go/No-Go

The last dangerous moment is right before apply. An AI needs one packet that says whether approval, privacy, bundle verification, review, rollback readiness, and live Studio readiness are all in the right state before it mutates anything.

Implemented next: `rs autopilot live-gate` writes `live-gate.json` and `live-gate.md` from a run or session. It refreshes review-pack, approval, privacy, rollback, and bundle verification, optionally checks live bridge readiness, records every gate check, and only exposes an apply action when `--approved` and live readiness pass.

Why: This gives the AI a hard mutation boundary instead of relying on memory, chat intent, or scattered artifacts.

## 20. Honest Closeout

The final AI failure mode is overclaiming: saying a feature is done because the plan exists or tests passed offline. A Roblox-building AI needs a completion artifact that refuses to call work done until live apply, rollback, playtest, proof, judgment, and creator acceptance all agree.

Implemented next: `rs autopilot closeout` writes `closeout.json` and `closeout.md`. It refreshes proof, acceptance, judgment, review-pack, privacy, and rollback artifacts, then emits a done/not-done verdict, safe-to-say claims, do-not-say claims, blockers, warnings, and next actions.

Why: This gives the AI a trustworthy closing ritual: it can report progress without pretending offline readiness is live completion.

## 21. Run Timeline / Black Box

Even with closeout, a resumed AI can waste time rereading a pile of JSON files or miss stale evidence. The product needs a flight recorder for each run: one timeline that says what exists, what is missing, what is stale, and exactly where to resume.

Implemented next: `rs autopilot timeline` writes `timeline.json` and `timeline.md`. It walks the known run artifacts in lifecycle order, captures each packet's status, flags missing required artifacts, warns when proof or gate packets are older than newer evidence, and emits a single safe `resumeCommand`.

Why: This turns a run folder into an immediately understandable handoff surface instead of an archaeological dig.

## 22. Safe Offline Drive

The next leap is reducing orchestration friction. An AI should not need to remember whether to run start, approval, closeout, live-gate, proof, and timeline in the correct order. It needs a safe drive mode that does all non-mutating work and stops exactly where human approval or live Studio proof begins.

Implemented next: `rs autopilot drive` writes `drive.json` and `drive.md`. It bootstraps a prompt when needed, refreshes live-gate, closeout, and timeline artifacts for existing runs, records every stage it performed, and exposes one `resumeCommand` while refusing to run live apply.

Why: This gives an AI one high-leverage command for "do everything safe now, then tell me the next honest boundary."

## 23. Pitch Board

An AI should not always pick the first plausible interpretation. For game creation, the wow moment is offering a few buildable directions, explaining tradeoffs, and letting the creator choose before the agent commits to an implementation path.

Implemented next: `rs autopilot pitch` writes `pitch.json` and `pitch.md`. It turns a prompt into ranked deterministic recipe stacks, scores each candidate, explains why it fits, lists acceptance criteria, and provides exact `drive` and `kickoff` commands without creating run folders or mutating Studio.

Why: This gives the AI creative taste while keeping every option grounded in known, verifiable build primitives.

## 24. Player-Facing Storyboard

After choosing a direction, the AI still needs to explain the experience like a game designer, not a build system. A creator should see the player promise, core loop, UI surfaces, and demo steps before approving mutation.

Implemented next: `rs autopilot storyboard` writes `storyboard.json` and `storyboard.md`. It maps a prompt or run folder to player-facing beats, UI surfaces, systems, acceptance criteria, and proof-backed demo steps without creating a run or touching Studio.

Why: This gives the AI a concrete language for "what this game will feel like" while staying tied to verifiable recipes and evidence artifacts.

## 25. Creator Proposal Packet

The AI needs one artifact it can present to the creator before doing safe offline build work. That packet should combine recommendation, alternatives, storyboard, exact next commands, safe claims, and forbidden claims.

Implemented next: `rs autopilot proposal` writes `proposal.json` and `proposal.md` plus companion pitch/storyboard packets. It recommends the best deterministic direction, keeps alternatives visible, describes the player experience, and tells the AI exactly what it may and may not claim.

Why: This turns scattered planning surfaces into a clean creator-review moment before the agent drives a run.

## 26. Creator Selection Memory

After a proposal, the AI needs durable memory of what the creator actually chose. Chat context is too fragile; the selected candidate, safe claims, forbidden claims, and next command should become a local artifact before any build orchestration starts.

Implemented next: `rs autopilot select` reads `proposal.json`, records the recommended candidate or an explicit `--candidate`, and writes `selection.json` plus `selection.md` without creating run folders or mutating Studio.

Why: This gives the agent a trustworthy handoff from creative review into build execution, even after context loss or agent handoff.

## 27. Selection-Driven Launch

Once the creator has chosen a proposal, the AI should not have to copy a generated `drive` command by hand. The product should consume the selection artifact, run the safe offline drive path, and leave a launch receipt that says exactly where the build stopped.

Implemented next: `rs autopilot launch` reads `selection.json`, reconstructs the selected drive invocation, writes `launch.json` and `launch.md`, refreshes the selected run's `drive.json`, and stops before Studio mutation. Its command queue is intentionally short and non-mutating so the next AI sees the real boundary instead of a noisy downstream backlog.

Why: This closes the gap between "approved direction" and "safe offline work completed" while preserving the approval boundary.

## 28. Self-Starting Live Readiness

An AI should not stall on "bridge unreachable" when the CLI already knows how to start the local bridge safely. The readiness gate should bootstrap the bridge, then report the true remaining blocker: no Studio session, plugin mismatch, missing capabilities, or approval state.

Implemented next: `rs autopilot ready` and live-gate readiness now call the existing local bridge auto-spawner before polling Studio sessions. If the bridge starts but Studio is not connected, the report moves to explicit Studio/plugin fixes instead of telling the user to manually start the bridge.

Why: This makes live-readiness diagnosis feel like a helpful operator, not a brittle socket check.

## 29. Setup Repair Packet

Once readiness can diagnose the real blocker, the AI needs a durable packet it can show the creator: what is wrong, what to run, what to restart, what is safe to say, and what is still unproven.

Implemented next: `rs autopilot setup` writes `setup.json` and `setup.md` from the readiness gate. It surfaces Studio/plugin blockers, keeps live mutation forbidden, and gives exact commands for install, doctor fix, setup retry, and final readiness retry. With `--fix`, it runs the existing install-plugin flow and records the installed bundle/hash plus the Studio windows that must restart.

Why: This turns "I cannot connect" into an operator-grade repair checklist an AI can follow across sessions.

## 30. AI Companion Packet

The AI needs one opening artifact that answers both creative and operational questions: what should we build, what should the creator approve, is Studio ready, what is blocked, and what must not be claimed yet.

Implemented next: `rs autopilot companion` writes `companion.json` and `companion.md` beside `companion-proposal.*` and `companion-setup.*`. It recommends a proposal candidate, preserves setup readiness, lists exact select/launch/setup actions, and refuses to pretend the creator selected or launched anything before those artifacts exist. With `--fix`, it can perform the local plugin install step before writing the setup packet.

Why: This gives a fresh AI one trustworthy first move for Roblox game building instead of making it stitch together proposal and readiness state from chat memory.

## 31. Static Player-Journey Simulation

An AI needs one more check before asking for live apply: can a player actually traverse the planned slice on paper? The CLI should dry-run the intended journey from static plan artifacts instead of waiting for a human to discover that the generated systems are disconnected.

Implemented next: `rs autopilot simulate` writes `simulation.json` and `simulation.md` for a run or plan. It checks arrival, first interaction, reward feedback, progression, UI feedback, server authority, and evidence handoff, then returns patch/playtest/certify actions. `kickoff` now writes simulation artifacts before bundling, and certification includes the static playability signal as a warning-grade gate.

Why: This gives an AI a fast, honest "will this feel playable?" preflight before crossing the live Studio boundary.

## 32. Feature Graph

An AI should not have to infer system structure from a flat list of operations. It needs a compact graph of recipes, generated sources, script targets, remotes, UI, and verification gates before it can safely patch or explain a run.

Implemented next: `rs autopilot graph` writes `feature-graph.json` and `feature-graph.md`. It connects plan operations into nodes and edges, links generated sources to scripts, detects remote references from generated Lua, refreshes the bundle when run separately, and is included in kickoff plus planner-pack context.

Why: This gives the model a working mental map of the game slice before it edits, repairs, or explains it.

## 33. Balance Intelligence

An AI needs to catch broken economy pacing before a creator spends time applying and playtesting a slice. Shops, coins, tycoons, and upgrades should expose enough static evidence for the model to see whether players can earn, spend, and progress.

Implemented next: `rs autopilot balance` writes `balance.json` and `balance.md`. It extracts currency names, reward values, prices, starter balances, and first-purchase pacing from tuned manifests plus generated Luau, refreshes the bundle when run separately, and is included in kickoff, certification, timeline, and planner-pack context.

Why: This gives the model an economy sanity check that sits between structural graph review and live playtest evidence.

## 34. Impact / Blast-Radius Map

An AI needs to understand what a live apply could touch before it asks for permission. Scripts, remotes, cloud services, deletes, and asset uploads should be visible as a service-by-service blast-radius map, not buried inside a flat operation list.

Implemented next: `rs autopilot impact` writes `impact.json` and `impact.md`. It maps touched services, scripts, remotes, operation groups, cloud/persistence surfaces, generated sources, approval pressure, and rollback requirements, refreshes the bundle when run separately, and is included in kickoff, certification, timeline, refresh, and planner-pack context.

Why: This gives the model a precise "what could I affect?" artifact before approval, live-gate, or apply.

## 35. Remote Contract Map

An AI needs to know the generated client/server API before it patches gameplay. RemoteEvents and RemoteFunctions should be visible as contracts with callers, handlers, listeners, and weak spots rather than hidden in script text.

Implemented next: `rs autopilot contracts` writes `contracts.json` and `contracts.md`. It maps remotes to generated scripts, classifies client calls, server handlers, server emits, and client listeners, flags one-sided or weakly validated contracts, refreshes the bundle when run separately, and is included in kickoff, certification, timeline, refresh, and planner-pack context.

Why: This gives the model a concrete RemoteEvent/RemoteFunction contract sheet before repair, approval, or live apply.

## 36. Server Authority Audit

An AI needs to know whether generated gameplay-critical state is actually server-owned. Remote contracts can exist while the exploit surface is still unclear, so the CLI should summarize client/server script sides, DataStore access, client mutation risk, and profile hooks in one go/no-go artifact.

Implemented next: `rs autopilot authority` writes `authority.json` and `authority.md`. It audits generated surfaces, flags client-side persistence and state mutation risks, imports weak remote-contract findings, refreshes the bundle when run separately, and is included in kickoff, certification, timeline, refresh, and planner-pack context.

Why: This gives the model a server-authority score before it asks for approval or applies multiplayer-sensitive code.

## 37. Player UX Audit

An AI needs to know whether the generated slice is understandable to a player, not only safe to apply. The product should detect visible UI, actionable controls, feedback loops, onboarding copy, and readable text before the agent asks for live apply.

Implemented next: `rs autopilot ux` writes `ux.json` and `ux.md`. It audits generated UI and world prompt surfaces, interaction handlers, feedback signals, onboarding copy, and readable text evidence, refreshes the bundle when run separately, and is included in kickoff, certification, timeline, refresh, and planner-pack context.

Why: This gives the model a player-facing "will they know what to do?" check before approval or live playtest.

## 38. Copy Deck / Localization Readiness

An AI needs the exact player-facing words it generated before it can tune tone, explain UI, localize labels, or ask a creator to review copy. Those strings should be extracted into one artifact instead of being scattered across plan properties and Luau source.

Implemented next: `rs autopilot copy-deck` writes `copy-deck.json` and `copy-deck.md`. It extracts UI labels, buttons, prompts, status text, and feedback strings from plan properties plus generated Luau, marks dynamic strings, refreshes the bundle when run separately, and is included in kickoff, certification, timeline, refresh, trace, and planner-pack context.

Why: This gives the model a readable text inventory for polish and localization before approval or live playtest.

## 39. Performance Budget Audit

An AI needs a fast static answer to "is this slice likely too heavy?" before asking the creator to apply it. Generated scripts, remotes, loops, waits, persistence calls, and instance counts should be budgeted in one artifact rather than discovered only after Play Solo feels slow.

Implemented next: `rs autopilot performance` writes `performance.json` and `performance.md`. It estimates planned instance and script counts, source size, loop/frame-step patterns, wait/delay calls, remote references, async fanout, and DataStore references, refreshes the bundle when run separately, and is included in kickoff, certification, timeline, refresh, trace, and planner-pack context.

Why: This gives the model a lightweightness check before approval, live apply, or performance-sensitive repair work.

## 40. Accessibility Review

An AI needs to know whether generated UI is legible and usable before it claims the feature is player-ready. Touch targets, scaled text, contrast, input modality, and motion-sensitive patterns should be checked in one pre-apply artifact.

Implemented next: `rs autopilot accessibility` writes `accessibility.json` and `accessibility.md`. It audits generated UI surfaces for scalable text, touch target size, text/background contrast, input affordance signals, mouse-only handlers, and motion-sensitive generated source patterns, refreshes the bundle when run separately, and is included in kickoff, certification, timeline, refresh, trace, and planner-pack context.

Why: This gives the model an inclusive-design check before approval, handoff, or live Play Solo review.

## 41. Policy / Safety Audit

An AI needs to know when generated content touches Roblox policy-sensitive surfaces before it asks for approval, live apply, or user-facing publication claims. Purchases, persistence, teleports, HTTP, chat, personal data requests, randomized rewards, and off-platform links should be surfaced in one artifact instead of hidden in plan text or generated Luau.

Implemented next: `rs autopilot policy` writes `policy.json` and `policy.md`. It scans plan properties plus generated source for policy-sensitive signals, classifies blocking off-platform/personal-data findings separately from review-required warning surfaces, refreshes the bundle when run separately, and is included in kickoff, certification, timeline, refresh, trace, and planner-pack context.

Why: This gives the model a Roblox policy/safety preflight before approval, handoff, or live Play Solo review.

## 42. Intent Traceability Matrix

An AI needs one artifact that proves the creator request did not get lost between prompt, recipes, generated files, and review gates. Without traceability, a run can be valid, bundled, and even ready to apply while still missing the user's actual requested feature.

Implemented next: `rs autopilot trace` writes `trace.json` and `trace.md`. It maps the prompt to expected recipes, checks present recipes and generated files, verifies core offline review artifacts including impact, contracts, authority, UX, copy deck, performance, accessibility, policy, style guide, world blueprint, onboarding, showcase, telemetry, monetization, social, liveops, and asset brief, refreshes the bundle when run separately, and feeds kickoff, certification, proof, acceptance, timeline, and planner-pack context.

Why: This gives the model a prompt-to-artifact checklist it can cite before handoff, repair, acceptance, or user-facing status.

## 43. Offline Packet Refresh

An AI should not have to remember the regeneration order for source audit, simulation, graph, balance, impact, contracts, authority, UX, copy deck, performance, accessibility, policy, style guide, world blueprint, onboarding, asset brief, showcase, telemetry, monetization, social, liveops, trace, bundle, handoff, certification, planner pack, critique, and playtest artifacts. After any patch or resumed session, the product should heal stale review packets with one safe command.

Implemented next: `rs autopilot refresh` writes `refresh.json` and `refresh.md`. It rebuilds the derived offline review packet in dependency order, preserves honest blockers and warnings, refreshes bundle hashes, and gives the next verification/readiness commands.

Why: This gives the model a deterministic "make my evidence current" button before handoff, approval, live readiness, or user-facing reporting.

## 44. Asset Production Brief

An AI needs more than scripts and primitive parts to help a creator build a Roblox game that feels real. It needs a concrete list of UI images, 3D props, audio cues, VFX textures, thumbnails, generation prompts, acceptance checks, and import/upload commands tied to the actual run.

Implemented next: `rs autopilot asset-brief` writes `asset-brief.json` and `asset-brief.md`. It scans recipes, plan operations, prompt text, generated Luau, the style guide, and the world blueprint, groups asset requests by kind, marks high-priority player-facing art, emits generation prompts and acceptance checks, refreshes the bundle when run separately, and is included in kickoff, certification, timeline, refresh, trace, and planner-pack context.

Why: This lets the model move from "the game technically exists" to "here is the exact production shopping list that would make it inspectable, playable, and presentable."

## 45. Style Guide / Creative Contract

An AI needs a durable creative contract before it generates UI images, props, audio, thumbnails, copy, and future patches. Without it, every resumed session can drift into a different palette, tone, asset style, or Roblox genre assumption.

Implemented next: `rs autopilot style-guide` writes `style-guide.json` and `style-guide.md`. It infers theme, genre, tone, palette, visual rules, UI rules, copy rules, audio rules, and reusable asset prompts from the creator request, recipe evidence, and plan, refreshes the bundle when run separately, and is included in kickoff, certification, timeline, refresh, trace, and planner-pack context.

Why: This gives the model a stable style bible so every future asset, label, thumbnail, and patch feels like part of the same Roblox game.

## 46. World Blueprint / Spatial Contract

An AI needs a durable map of where the generated Roblox game actually happens. Scripts, remotes, and assets are not enough; the model needs zones, player routes, interaction anchors, camera shots, and spatial build rules so future patches, screenshots, playtests, and thumbnails align to one readable world.

Implemented next: `rs autopilot world-blueprint` writes `world-blueprint.json` and `world-blueprint.md`. It infers a zone map, route steps, interaction anchors, camera shots, build rules, and proof hints from the creator request, recipes, plan paths, and style direction, refreshes the bundle when run separately, and is included in kickoff, certification, timeline, refresh, trace, and planner-pack context.

Why: This gives the model a spatial contract, so it can stop guessing where the player spawns, walks, buys, collects, fights, screenshots, and proves the game loop.

## 47. Onboarding / First-Session Contract

An AI needs to know how the generated game teaches itself. A valid plan can still fail the player if the first 90 seconds do not explain the goal, first action, reward feedback, progression, and proof that the player understood the loop.

Implemented next: `rs autopilot onboarding` writes `onboarding.json` and `onboarding.md`. It turns recipes and world zones into first-session steps, teaching prompts, feedback expectations, and Play Solo proof checks, refreshes the bundle when run separately, and is included in kickoff, certification, timeline, refresh, trace, and planner-pack context.

Why: This gives the model a concrete teaching contract before it asks for playtest evidence or tells the creator the game is understandable.

## 48. Showcase / Creator Demo Contract

An AI needs to know how to reveal the generated Roblox game, not just how to validate it. A technically correct run can still feel unimpressive if the model cannot plan the hero screenshot, thumbnail promise, trailer beats, talking points, and proof captures that make the work understandable to the creator.

Implemented next: `rs autopilot showcase` writes `showcase.json` and `showcase.md`. It turns style, world blueprint, onboarding, recipes, and proof requirements into demo shots, thumbnail direction, trailer clips, creator talking points, publish-prep checks, and exact `record-playtest` evidence commands, refreshes the bundle when run separately, and is included in kickoff, certification, timeline, refresh, trace, and planner-pack context.

Why: This gives the model a presentation contract so it can show the generated slice with confidence while still refusing to invent live proof that has not been captured.

## 49. Telemetry / Retention Contract

An AI needs a learning loop after the generated Roblox slice exists. Without an event and funnel contract, the model can build a game that looks good in review but has no way to answer whether players found the first action, earned the first reward, bought the first upgrade, or had a reason to return.

Implemented next: `rs autopilot telemetry` writes `telemetry.json` and `telemetry.md`. It maps recipes, onboarding, balance evidence, and generated systems into anonymous analytics events, first-session funnels, retention hooks, product questions, privacy guardrails, and next commands, refreshes the bundle when run separately, and is included in kickoff, certification, timeline, refresh, trace, and planner-pack context.

Why: This gives the model a retention-aware measurement contract before it adds analytics code, tunes economy pacing, or claims the generated game is ready to learn from players.

## 50. Monetization / Commerce Trust Contract

An AI needs to reason about commerce without overstepping into unsafe marketplace claims. A generated shop, tycoon, collectible, or obby can suggest paid value, but the model must know what can be sold, where it appears, what review evidence is required, and what must stay free before any Roblox product IDs or MarketplaceService code are introduced.

Implemented next: `rs autopilot monetization` writes `monetization.json` and `monetization.md`. It maps recipes, balance evidence, policy review, telemetry, and generated progression/shop surfaces into offer candidates, commerce surfaces, price-test ideas, review inputs, trust guardrails, and next commands, refreshes the bundle when run separately, and is included in kickoff, certification, timeline, refresh, trace, and planner-pack context.

Why: This gives the model a commerce contract that preserves player trust, avoids invented product IDs, and blocks live revenue claims until creator approval, configured Roblox IDs, receipt handling, Studio apply, and playtest proof exist.

## 51. Social / Growth Trust Contract

An AI needs to reason about Roblox-native social play without becoming spammy or making unproven growth claims. Friend visits, co-op moments, badges, leaderboards, and community hooks can make a game feel alive, but they must remain optional, solo-safe, policy-safe, and proof-gated.

Implemented next: `rs autopilot social` writes `social.json` and `social.md`. It maps recipes, world layout, onboarding, telemetry, and policy evidence into social loops, optional invite moments, community hooks, proof checks, and guardrails, refreshes the bundle when run separately, and is included in kickoff, certification, timeline, refresh, trace, and planner-pack context.

Why: This gives the model a growth-aware social contract before it adds friend visits, parties, community events, badges, leaderboards, or social copy, while blocking auto-invites, off-platform prompts, friend-gated core progress, and live growth claims until implemented and proven.

## 52. Live Operations / Update Cadence Contract

An AI needs to help a Roblox game survive after the first playable slice. It should know what to update, when to run events, which experiments are safe, which proof gates block promotion, and when to refuse live claims.

Implemented next: `rs autopilot liveops` writes `liveops.json` and `liveops.md`. It maps recipes, telemetry, social, monetization, showcase, policy, and asset evidence into update cadence, event hooks, experiments, proof gates, operating rules, and next commands, refreshes the bundle when run separately, and is included in kickoff, certification, timeline, refresh, trace, and planner-pack context.

Why: This gives the model an operations contract for content drops, weekend events, shop refreshes, and retention experiments without claiming publishing, analytics, rollback, or playtest proof exists before the artifacts prove it.

## 53. Persistence / Data Safety Contract

An AI needs to protect player progress before it promises saves, profiles, inventories, upgrades, quests, or shop entitlements. It should know the DataStore schema, save/load flows, migration plan, server-authority boundary, and the proof needed to avoid data-loss claims.

Implemented next: `rs autopilot persistence` writes `persistence.json` and `persistence.md`. It maps recipes, generated source, balance, authority, telemetry, policy, and apply evidence into data models, server-authoritative flows, schema migrations, proof checks, guardrails, and next commands, refreshes the bundle when run separately, and is included in kickoff, certification, timeline, refresh, trace, and planner-pack context.

Why: This gives the model a save-data contract before it adds DataStore code or tells a creator that progress is safe, while blocking claims until API services, approved apply, reload playtest evidence, rollback evidence, and server authority support them.

## 54. Live Evidence Review / Diagnosis Contract

An AI needs to understand what live playtest evidence actually means before it patches, reports status, or claims a run is healthy. Raw screenshots, notes, and scenario results should become scenario-level observations, repair hypotheses, and claim boundaries.

Implemented next: `rs autopilot evidence-review` writes `evidence-review.json` and `evidence-review.md`. It reads the playtest plan, evidence kit, and recorded playtest result, then produces reviewed scenarios, observations, root-cause hypotheses, safe-to-say claims, do-not-say claims, and exact next commands for repair, judgment, or health.

Why: This gives the model a diagnosis layer between "I saw something in Play Solo" and "I know what to fix or what I can honestly tell the creator."

## 55. Studio Path Reconciliation / Drift Contract

An AI needs to know whether the current Studio place still matches the run it is about to discuss, repair, or extend. It should compare planned and applied paths against fresh context/survey evidence instead of assuming a previous apply is still present.

Implemented next: `rs autopilot reconcile` writes `reconcile.json` and `reconcile.md`. It reads `plan.json`, optional `apply.json`, and supplied or discovered `context.json` / `survey.json`, then reports matched planned paths, missing paths, survey findings, safe zones, `aligned`, `plannedNotApplied`, `needsSurvey`, `needsReview`, or `driftDetected` status, and exact next commands.

Why: This gives the model a continuity check between "the run folder says it happened" and "the open Studio place still appears to contain it," which is crucial after restarts, manual edits, failed applies, or resumed sessions.

## 56. Roblox Publish Prep / Launch Dossier

An AI needs a careful bridge from "the slice works locally" to "the creator can consider Roblox-facing release work." It should draft store copy, thumbnail needs, launch notes, and release blockers from evidence rather than pretending that local artifacts mean the experience is published.

Implemented next: `rs autopilot publish-prep` writes `publish-prep.json` and `publish-prep.md`. It reads the run plan, bundle verification, showcase, policy, privacy, reconciliation, health, closeout, proof, acceptance, rollback, and asset brief evidence, then emits store title/description drafts, update notes, store asset needs, checklist status, safe-to-say claims, do-not-say claims, blockers, and exact next commands.

Why: This gives the model a launch-facing dossier that is useful to a creator while still refusing to claim publishing, metadata upload, release-candidate readiness, or live proof until the artifacts actually support it.

## 57. Creator Feedback Triage

An AI needs a disciplined way to convert subjective creator notes into specific next work. It should categorize feedback, ask clarifying questions when notes are vague, and route concrete notes into existing offline patch/review commands instead of treating feedback as loose chat.

Implemented next: `rs autopilot feedback` writes `feedback.json` and `feedback.md`. It reads the run plan plus known review artifacts, accepts repeated `--note` values, categorizes each note by UX, copy, showcase, repair, balance, policy, performance, gameplay, publish, or planning, then emits severity, confidence, patch lanes, clarification questions, safe-to-say claims, do-not-say claims, blockers, and exact next commands.

Why: This gives the model a creator-review loop that stays honest: missing or vague feedback is blocked, concrete feedback becomes a patch plan, and no one claims implementation before a patch is generated and verified.

## 58. Feedback Patch Work Order

An AI needs a clean handoff from triaged feedback into a strict patch-generation task. It should not make the next model reread loose notes, infer safety rules, or guess the adopt/certify sequence.

Implemented next: `rs autopilot feedback-patch` writes `feedback-patch.json` and `feedback-patch.md`, plus `feedback-planner-pack.json` and `feedback-planner-pack.md` by default. It consumes `feedback.json`, produces lane-specific acceptance checks, builds a custom strict planner prompt, records the exact `adopt-plan` command, lists validation commands, refreshes the bundle, and preserves safe-to-say / do-not-say guardrails.

Why: This gives the next model a precise patch work order while still refusing to claim the patch exists, was applied, was playtested, or was approved.

## 59. Claim Check / Response Guard

An AI needs a final truth gate before it speaks. It should be able to test the exact sentence it wants to tell the creator against current artifacts and get a supported, risky, or blocked verdict with safer wording.

Implemented next: `rs autopilot claim-check` writes `claim-check.json` and `claim-check.md` from repeated `--claim` values. It reads proof, acceptance, health, rollback, publish-prep, feedback, feedback-patch, user brief, cockpit, review pack, privacy, and bundle evidence, then classifies each claim, cites artifacts, blocks unsupported live/publish/production assertions, and refreshes the bundle.

Why: This makes honest reporting enforceable. The model can be ambitious in planning while still proving every creator-facing claim before it says it.

## 60. Evidence-Gated Creator Response

An AI needs help turning verified facts into a polished user update. It should not have to manually assemble a final message from proof, brief, and claim-check artifacts while trying not to overclaim.

Implemented next: `rs autopilot respond` writes `response.json` and `response.md`. It refreshes `user-brief.json`, runs `claim-check` on explicit or generated safe claims, writes the exact creator-facing message, records safe-to-say and do-not-say lists, refreshes the bundle, and exits non-zero unless every checked claim is ready to send.

Why: This closes the last inch between evidence and communication. The model can give the creator a clear update while the CLI blocks unsupported production, publish, playtest, rollback, or live-apply claims.

## 61. Creator Decision Ledger

An AI needs durable memory for what the creator chose, constrained, rejected, or noted. It should not rely on chat scrollback, and it must not confuse "I like this direction" with approval to mutate live Studio.

Implemented next: `rs autopilot decision` writes `decisions.json` and `decisions.md` from repeated `--decision`, `--constraint`, `--rejection`, and `--note` values. It records active constraints and rejected directions, refreshes the bundle, feeds safe decision claims into `claim-check`, and exits non-zero when a note tries to approve live apply, publish, upload, rollback, release, or production work.

Why: This gives the model creator intent that survives context loss while keeping the live mutation boundary crisp.

## 62. Decision Alignment Gate

An AI needs to know whether its current plan still respects the creator's recorded choices. It should catch obvious rejected-direction drift before another model spends time patching, adopting, or asking for live apply.

Implemented next: `rs autopilot align` writes `alignment.json` and `alignment.md`. It reads `plan.json`, generated sources, and `decisions.json`, reports active constraints, detects rejected-direction terms, refreshes the bundle, and feeds a safe static-audit claim into `claim-check`.

Why: This turns the decision ledger from passive memory into a preflight gate for future agents, while still refusing to claim implementation or live Studio state from static text alone.

## 63. Alignment-Aware Next Move

An AI needs its "what should I do now?" command to honor creator decisions automatically. It should not recommend critique, certification, or live readiness while a recorded decision has not been aligned or while alignment is blocked.

Implemented next: `rs autopilot next` now reads `decisions.json` and `alignment.json`. When decisions exist without alignment, it recommends `rs autopilot align`; when alignment is blocked, it returns `resolveDecisionDrift` before any other continuation. `rs autopilot roadmap` now treats that blocked next decision as a roadmap blocker instead of burying it as a warning.

Why: This makes creator intent part of the command queue, not a side note a model has to remember.

## 64. Decision-Aware Handoff Packets

An AI needs the one-screen cockpit and copy/paste capsule to include creator decisions and alignment explicitly. Otherwise a fresh model can read the status packet but miss the most important continuity rule: what the creator chose or rejected.

Implemented next: `rs autopilot control`, `cockpit`, and `capsule` now surface `decisions.json` and `alignment.json` when present. Cockpit adds Creator Decisions and Decision Alignment panels, while capsule marks both artifacts as required context in the resume prompt.

Why: This makes creator intent unavoidable at handoff time, where context loss hurts most.

## 65. AI Work Journal

An AI needs durable memory for what it tried, what it learned, which commands were attempted, and which evidence paths matter next. Otherwise a new session has to reconstruct the last model's work from scattered artifacts or chat scrollback.

Implemented next: `rs autopilot journal` writes `journal.json` and `journal.md` from repeated `--entry`, `--command`, `--result`, and `--evidence` values. It summarizes existing run artifacts, refreshes the bundle, feeds one safe continuity claim into `claim-check`, and routes live-mutation wording back to approval/live-gate guardrails instead of treating notes as proof.

Why: This gives resumed agents a reliable work log without weakening the evidence model. The journal remembers effort; proof and claim-check still decide what can be claimed.

## 66. Opportunity Map

An AI needs strategic judgment, not just a single next command. It should be able to compare build, repair, proof, handoff, and continuity moves, explain why each matters, and know what artifacts each move should produce.

Implemented next: `rs autopilot opportunities` writes `opportunities.json` and `opportunities.md`. It reads memory, next, roadmap, selected run artifacts, decisions, alignment, and journal state, then ranks opportunities with scores, confidence, commands, expected artifacts, evidence, blockers, safe claims, and forbidden claims.

Why: This helps the model choose the highest-leverage work instead of blindly taking the first command. It keeps ambition grounded in evidence and still refuses to treat ranking as implementation, approval, or production readiness.

## 67. AI Work Order

An AI needs a bridge from "best opportunity" to "do this exact thing now." It should not have to reinterpret the opportunity map into commands, validation, stop conditions, and user-facing claim boundaries by hand.

Implemented next: `rs autopilot work-order` writes `work-order.json` and `work-order.md`. It refreshes `opportunities.json`, selects the top opportunity or a named `--opportunity`, records objective, execution steps, validation commands, expected artifacts, stop conditions, safe claims, forbidden claims, and next actions, then refreshes the bundle when a run is selected.

Why: This turns strategic judgment into an executable, evidence-gated work packet. It lets the AI act decisively while preserving the rule that no work is claimed until commands run and proof/claim-check support the exact update.

## 68. AI Work Check

An AI needs a ritual after execution, not just before it. Once a work order exists, the next failure mode is saying "done" because the instructions were clear, even though the expected artifacts were never produced or the bundle drifted.

Implemented next: `rs autopilot work-check` writes `work-check.json` and `work-check.md`. It reads `work-order.json`, verifies the selected command, checks file-like expected artifacts, marks stdout or wildcard evidence for manual review, verifies the bundle, inspects journal continuity, refreshes the bundle, and returns `readyForClaimCheck` only when the artifact surface is present.

Why: This closes the loop between assignment and reporting. The AI gets a concrete evidence gate that says whether it can move to claim-check, must execute the selected command, or must stop for missing/manual evidence.

## 69. AI Cycle Packet

An AI needs one heartbeat command that says where the current run is in the work loop. Without it, the agent has to remember whether it is choosing, executing, checking, rewriting, responding, or handing off, which is exactly where stale chat context creates mistakes.

Implemented next: `rs autopilot cycle` writes `cycle.json` and `cycle.md`. It refreshes opportunities, creates a work order when missing, refreshes the bundle before work-check, writes work-check, claim-checks supported evidence, composes response artifacts when safe, and emits one status such as `executeWorkOrder`, `recordManualEvidence`, `rewriteClaim`, or `readyToReport`.

Why: This makes the CLI feel like an AI operating console instead of a bag of commands. The model gets one reliable loop state, exact next action, checked response boundary, and do-not-say list without running arbitrary shell commands or mutating Studio.

## 70. AI Failure Diagnosis

An AI needs a way to recover when the heartbeat says "stuck" or a command fails. Otherwise the next model has to infer whether it needs to run the selected command, record manual evidence, refresh a bundle, rewrite a claim, or stop for privacy/alignment blockers.

Implemented next: `rs autopilot diagnose` writes `diagnosis.json` and `diagnosis.md`. It reads cycle, work-check, claim-check, journal, bundle, playtest, alignment, and privacy evidence, accepts explicit `--command`, `--result`, `--error`, and `--evidence` inputs, then classifies incidents with exact recovery commands and claim boundaries.

Why: This gives the AI a durable failure router. It can be honest about what is broken, preserve failed command context, and continue from evidence instead of guessing from terminal fragments.

## 71. Safe Offline Action Runner

An AI needs a trusted hand between "this is the next command" and "the command ran and the loop evidence changed." Copying terminal fragments by hand is where agents accidentally cross live boundaries, skip journaling, or forget to rerun the cycle.

Implemented next: `rs autopilot act` writes `act.json` and `act.md`. It selects the next command from diagnosis, cycle, or work-order evidence, executes only whitelisted offline Autopilot actions through internal handlers, refuses live/mutating commands such as apply, live-gate, ready, setup, upload, smoke, bridge, or plugin repair, then refreshes cycle, diagnosis, and bundle evidence.

Why: This turns the CLI from a dashboard into a guarded co-worker. The model can make progress with one safe action at a time while the tool preserves artifacts and keeps Studio mutation behind explicit approval.

## 72. Guarded Offline Loop

An AI needs bounded momentum. Once one safe action can run, the next natural workflow is to keep cycling, acting, and refreshing evidence until the run is ready to report or a blocker appears, without asking the model to babysit each intermediate artifact.

Implemented next: `rs autopilot loop` writes `loop.json` and `loop.md`. It runs `cycle`, uses `act` for one whitelisted offline action, repeats up to `--max-steps`, stops at `readyToReport`, and records every iteration's cycle status, act status, command kind, artifacts, blockers, and next action. It still refuses live mutation through the same `act` boundary.

Why: This is the first real autonomous offline operator. It lets the AI turn a safe plan into checked response evidence while preserving a hard wall around Studio mutation, uploads, smoke, plugin repair, and unverified success claims.

## 73. Creator Message Router

An AI's next instruction usually arrives as ordinary chat, not a typed CLI command. The product should classify that message before the model guesses whether it is feedback, a new build request, a constraint, a status ask, a rollback ask, or live-approval wording.

Implemented next: `rs autopilot inbox` writes `inbox.json` and `inbox.md`. It redacts the message, selects the active run when possible, classifies intent, routes to safe commands such as `feedback`, `decision`, `rollback`, `cycle`, `respond`, or `start`, lists expected artifacts, and turns apply/publish/upload wording into approval/live-gate readiness instead of mutation permission.

Why: This is the conversational front door. It lets an AI respond to the creator's actual words with evidence-backed commands and guardrails, while preventing the most dangerous failure mode: treating "go ahead" in chat as live Studio authorization.

## 74. Safe Creator Message Handler

Once chat is routed, the AI still needs a safe hand that can do the obvious offline step without copying commands into a shell. A feedback note should become `feedback.json`; a constraint should become `decisions.json`; a status ask should refresh cycle/response evidence; an apply request should stop at gates.

Implemented next: `rs autopilot handle` writes `handle.json` and `handle.md`. It refreshes `inbox.json`, executes one supported non-mutating route such as feedback, decision, rollback, cycle/response, mission, or offline start, refreshes bundle evidence when a run is selected, and refuses to execute approval, live-gate, ready, apply, upload, publish, bridge, smoke, setup, or plugin repair routes.

Why: This turns creator chat into real offline progress while keeping the same hard wall around live Studio mutation. It is the difference between "the AI knows the next command" and "the AI safely handled the user's note and can prove what happened."

## 75. Durable Conversation State

An AI needs memory that is grounded in artifacts, not just chat scrollback. After messages are routed and handled, the next session needs to know what the creator asked, what the CLI actually did, what remains open, and what claims are safe.

Implemented next: `rs autopilot conversation` writes `conversation.json` and `conversation.md`. It reads inbox, handle, feedback, decisions, response, journal, loop, and cycle artifacts, summarizes creator/AI turns, lists open loops, carries safe-to-say and do-not-say guardrails, and recommends the next command without mutating Studio.

Why: This is the continuity layer for long-running AI game work. It lets a resumed model pick up the real state of the Roblox build without trusting stale chat, overstating handled feedback, or losing live-approval boundaries.

## 76. One-Message AI Chat Operator

An AI needs a single safe entrypoint for ordinary creator chat. The best workflow should not require the model to remember whether to call inbox, handle, feedback-patch, loop, conversation, or respond in the right order.

Implemented next: `rs autopilot chat` writes `chat.json` and `chat.md`. It routes and handles the creator message, prepares feedback patch work orders and planner packs when feedback is routed, runs bounded offline loop steps when appropriate, refreshes conversation state, and composes a checked response only when claim evidence supports the reply.

Why: This is the first "talk to the creator, do the safe work, and answer honestly" operator. It gives agents a one-command conversational workflow while preserving the same hard boundary around live Studio apply, upload, publish, setup repair, smoke, and plugin actions.

## 77. Creator Promise Fulfillment Audit

An AI needs a hard stop before saying "done." Proof, acceptance, and trace are valuable, but the model still needs one checklist that maps the creator's actual promise to present evidence, missing gaps, and live proof.

Implemented next: `rs autopilot fulfillment` writes `fulfillment.json` and `fulfillment.md`. It refreshes proof, acceptance, and trace, checks inferred recipes, generated sources, offline review artifacts, live apply, playtest result, rollback proof, and health proof, then returns `gapsFound`, `needsLiveProof`, or `fulfilled` with exact next actions.

Why: This is the answer to "can I honestly tell the creator this is complete?" It gives agents a concrete promise-to-artifact audit and prevents offline progress from being mistaken for live Roblox completion.

## 78. Creator Promise Satisfier

An AI needs a safe handoff from "this promised feature is missing" to "here is the patch run that adds it." Without that bridge, the model has to manually translate fulfillment gaps into recipe commands and can easily skip comparison, sequencing, or live-gate boundaries.

Implemented next: `rs autopilot satisfy` writes `satisfy.json` and `satisfy.md`. It refreshes fulfillment, selects missing deterministic recipe promises, creates an offline patch run, writes the full review packet, compares source vs. patch, writes an ordered sequence, and records live-gated next actions without mutating Studio.

Why: This closes the loop from promise audit to safe patch candidate. The AI can now move from "coins are missing" to a reviewable patch run and exact apply gate without pretending the live place changed.

## 79. Multi-Step Promise Loop

An AI needs to close the whole creator promise, not just one missing feature. If a request implies coins, quests, saves, and a shop, the agent should not manually repeat patch commands or lose the ordered source-plus-patch context between steps.

Implemented next: `rs autopilot promise-loop` writes `promise-loop.json` and `promise-loop.md`. It computes recipe coverage across the source run plus generated patches, creates offline patch runs for missing recipe batches, writes per-step comparisons, emits a final ordered sequence, and stops without mutating Studio.

Why: This turns fulfillment into a self-driving offline repair loop. A model can now say "the deterministic promise gaps are covered by this sequence" with artifact evidence, while still keeping live apply, playtest, rollback, and health proof separate.

## 80. Promise-Aware Command Queue

An AI should not need to remember that `promise-loop` exists. If the user asks for coins and quests but the active run only has a shop, the normal next-action queue should choose promise repair before generic critique, certification, or live readiness.

Implemented next: `rs autopilot next` now detects missing deterministic creator-promise recipes for the selected prompt, `opportunities` ranks that as fulfillment work, `cycle` can select it as the work order, and `act` can execute `fulfillment`, `satisfy`, and `promise-loop` through internal offline handlers.

Why: This makes creator promises part of the agent's muscle memory. The CLI now pulls the model toward the right repair loop instead of relying on the model to manually stitch together fulfillment, patching, and sequencing commands.

## 81. Live Demo Rehearsal Runbook

An AI needs one artifact that turns all of the proof packets into a real presentation path. Approval, live-gate, showcase, evidence, playtest, and closeout are strong individually, but a model still needs a single ordered script for "show the creator, ask for exact approval, verify readiness, apply only if GO, collect proof, and close out honestly."

Implemented next: `rs autopilot rehearsal` writes `rehearsal.json` and `rehearsal.md`. It refreshes showcase, evidence-kit, live-gate, and closeout, emits a creator script, readiness checks, runbook steps, evidence to collect, stop conditions, safe claims, and forbidden claims. The report itself is non-mutating while the apply step is explicitly marked as mutating. `next`, `judge`, and `act` now make rehearsal part of the normal AI path for ready offline runs.

Why: This is the CLI becoming the AI's stage manager. The model can lead a live Roblox demo with confidence because the dangerous boundary is visible, approval is exact, and every success claim has a post-apply proof step.

## 82. Cold-Start Orientation Packet

An AI model often joins midstream after context compaction, a handoff, or a long build session. It should not have to infer the right read order from scattered files, and it should not guess whether the current mode is build, fulfillment, approval, proof, or report.

Implemented next: `rs autopilot orient` writes `orientation.json` and `orientation.md`. It refreshes capsule, cockpit, and timeline artifacts, then emits session mode, read order, operating rules, safe claims, forbidden claims, trusted artifacts, a copy/paste model prompt, and one exact next command. `act`, timelines, bundle classification, artifact collectors, and opportunities now know how to refresh or surface orientation packets.

Why: This gives every fresh AI context a first page. The model can start by reading exactly what matters, inherit the repo's safety rules, avoid false completion claims, and continue the run without spelunking through the entire artifact tree.

## 83. Creator Preference Profile

An AI should get better at helping a specific creator over time. Decisions, constraints, rejected directions, feedback notes, proposal selections, and repeated prompt themes should become durable planning context, not disposable chat history.

Implemented next: `rs autopilot preferences` writes `creator-preferences.json` and `creator-preferences.md`. It learns explicit signals from `decisions.json`, `feedback.json`, `demo-learn.json`, proposal selections, run prompts, and recipe history, then emits constraints, rejections, learned demo preferences, prompt themes, feedback themes, recipe affinities, planning guidance, safe claims, and forbidden claims. Cockpit and orientation now surface this profile so fresh AI sessions read creator taste before proposing or continuing work.

Why: This makes the CLI feel like it knows the creator. The model can preserve style and boundaries across runs while still keeping preferences separate from approval, apply, playtest, rollback, and completion proof.

## 84. Cross-Run Game Bible

An AI can have memory and preferences and still accidentally make each patch feel like a new game. The product needs a canon layer: what is this project called, what player promise is sacred, what systems exist, what style/world rules must survive, and what proof is required before canon claims are safe.

Implemented next: `rs autopilot game-bible` writes `game-bible.json` and `game-bible.md`. It fuses project memory, creator preferences, architect/storyboard evidence, style guides, and world blueprints into one canon artifact with title, player promise, core loop, genre, tone, canon rules, style rules, world rules, system roles, continuity rules, proof contract, source runs, safe claims, and forbidden claims. Cockpit and orientation can surface it so new AI sessions preserve identity before adding more features.

Why: This makes the CLI a continuity partner, not just a task runner. The model can extend the same Roblox game over time, keep patches coherent, and still avoid pretending canon guidance is implementation proof.

## 85. Canon-Aware Creative Director

Memory and canon help an AI avoid drift, but they do not automatically answer "what would be the most valuable, exciting, and safe thing to build next?" The product needs a creative director layer that turns taste, project identity, and current opportunity evidence into a ranked slate of ambitious build bets.

Implemented next: `rs autopilot director` writes `director.json` and `director.md`. It refreshes the game bible and opportunity map, then emits strategic themes, recommended build bets, canon fit, exact safe offline commands, expected artifacts, proof needs, risks, constraints, anti-goals, safe claims, and forbidden claims. Cockpit, capsule, orientation, artifact classification, and safe offline `act` can now surface or refresh this strategy packet.

Why: This makes the CLI more than a guardrail system. It becomes an AI creative partner that can say "given who this game is, here are the next high-leverage bets and the proof path for each" without confusing taste, ambition, or strategy with implementation proof.

## 86. Director Pursuit

A ranked creative slate still leaves the AI doing manual command transfer. The product should let the AI choose a safe director bet and execute it through trusted internal code, while refusing unsupported or live-mutating commands.

Implemented next: `rs autopilot pursue` writes `pursuit.json` and `pursuit.md`. It refreshes the creative director, selects `--bet` or the first supported non-mutating offline bet, executes the whitelisted `kickoff` path through internal handlers, records the pursued run and artifacts, and turns unsupported/live/mutating bets into explicit blockers. Director next actions now route supported safe bets through `pursue` instead of asking the AI to hand-copy raw commands. It never uses shell passthrough.

Why: This is the first real move from "AI has taste" to "AI can safely act on that taste." The AI can propose the next ambitious game slice and immediately generate the offline evidence packet, while live Studio proof and creator approval stay guarded.

## 87. AI Work Agenda

After strategy, pursuit, cockpit, and control exist, the next problem is accountability: what is the AI responsible for right now, what proves each item, and where must it stop instead of freewheeling? A model needs a durable agenda that feels like a teammate's task board, not another pile of artifacts.

Implemented next: `rs autopilot agenda` writes `agenda.json` and `agenda.md`. It refreshes a cockpit snapshot under `agenda-context/`, distills the command queue into prioritized work items, labels each item by source and phase, records exact commands, expected artifacts, done-when checks, stop conditions, readiness flags, safe claims, and forbidden claims, and exposes a short next-action list. `rs autopilot act --source agenda` can now execute the first agenda item that has a safe internal handler, including guarded `pursue` and `agenda` refresh commands. It never mutates Studio or marks work complete by itself.

Why: This gives the AI a working memory for responsibility. It can tell "my first job is this, the proof is that, and I must stop if this gate fails" without scanning every packet or inventing completion criteria.

## 88. Agenda Sprint

An agenda is useful, but a best-friend AI should also be able to make bounded progress from that agenda without repeatedly asking itself which safe command to copy. The product needs a small offline sprint runner that advances the task board, records every action receipt, and stops exactly at blockers or live mutation gates.

Implemented next: `rs autopilot sprint` writes `sprint.json` and `sprint.md`. It refreshes agenda evidence, executes only agenda items that map to supported internal `act` handlers, skips commands already attempted in the same sprint, records per-step agenda and act artifacts, and stops at dry-run, blocker, unsupported action, live/mutating boundary, or `--max-steps`. It never shells out and never converts offline work into apply, playtest, publish, or production claims.

Why: This is where the AI starts to feel like a diligent collaborator instead of a dashboard reader. It can take the next few safe steps on its own, keep receipts, and know when to stop.

## 89. AI Work Retrospective

After an AI acts, it needs to understand what changed in its own evidence trail. Without a retrospective, the next session has to rediscover whether sprint executed anything, whether blockers remain, which claims are safe, and which live proof gates still matter.

Implemented next: `rs autopilot retrospect` writes `retrospective.json` and `retrospective.md`. It reads sprint, agenda, act, loop, cycle, diagnosis, proof, acceptance, health, privacy, journal, and bundle evidence, then records accomplishments, lessons, blockers, warnings, safe claims, forbidden claims, and exact next commands. It also verifies the bundle when present and preserves the rule that offline receipts are not live apply, playtest, rollback, publish, or production proof.

Why: This gives the AI a habit of learning from its own work. It can hand the next model a compact "here is what happened, here is what we learned, here is the next safe move" packet instead of relying on chat memory.

## 90. AI Operating Playbook

Memory, preferences, canon, and retrospectives are still separate artifacts. A fresh AI needs one project-level playbook that says how to behave on this Roblox game: which rules to preserve, what workflow to follow, what anti-patterns to avoid, and which proof gates are non-negotiable.

Implemented next: `rs autopilot playbook` writes `ai-playbook.json` and `ai-playbook.md`. It refreshes project memory, creator preferences, and the game bible, reads recent retrospectives, then writes operating principles, a default workflow, learned lessons, claim guardrails, anti-patterns, source runs, artifacts, and next actions. It turns scattered learning into a reusable operating manual without treating guidance as live apply approval or proof.

Why: This is the "how to be a good AI teammate here" layer. It gives future models the habits of the project, not just the facts.

## 91. Capability Atlas

A best-friend AI also needs tool self-knowledge. It should not infer commands, recipes, live boundaries, or artifact contracts from memory or long docs when the CLI can provide a verified atlas.

Implemented next: `rs autopilot capabilities` writes `capability-atlas.json` and `capability-atlas.md`. It combines deterministic recipes, use commands, AI workflows, key commands, expected artifacts, safe `act` handlers, supported operation kinds, required live plugin capabilities, examples, next actions, and safety boundaries. It is a tool-knowledge packet, not evidence that any feature has been built or applied.

Why: This lets a fresh model ask "what can this CLI actually do?" before acting. It reduces hallucinated commands, makes safe autonomy easier, and pairs naturally with the playbook: the playbook says how to behave; the atlas says what tools are real.

## 92. Tool-Aware Orientation

The atlas and playbook are strongest when a fresh model sees them before any run-specific dashboard. Cold-start orientation should not only say what to read; it should generate the tool map and operating habits it expects the model to obey.

Implemented next: `rs autopilot orient` now refreshes `capability-atlas.json` / `.md` and `ai-playbook.json` / `.md` beside `orientation.json`, marks both as required read-order items, and carries their paths in the orientation artifact map. Missing run roots remain warnings for empty-project startup instead of blocking a first build prompt.

Why: This turns orientation into the true first packet for an AI session: repo rules, verified tool affordances, project habits, cockpit status, and next command all in one place.

## 93. Command Sequence Guard

Once an AI has a command queue, the next risk is copying the wrong thing. A best-friend CLI should let the model ask, "Is this sequence real, offline-safe, act-supported, or does it cross a live/apply boundary?" before anything runs.

Implemented next: `rs autopilot command-guard` writes `command-guard.json` and `command-guard.md`. It accepts repeated `--command` values, a text file, or a run's existing orientation/cockpit/agenda/diagnosis/cycle/work-order queue, then classifies each command by command kind, known support, `act` support, expected artifacts, live-readiness requirements, mutation risk, and safer alternatives. Unsupported commands block; live or mutating commands route to setup, rehearsal, approval, ready/live-gate, rollback, and proof gates.

Why: This gives the AI a preflight for its own intentions. It can catch hallucinated subcommands and apply-boundary mistakes before they become terminal actions.

## 94. Execution Runbook

A command guard says whether a queue is safe; a best-friend AI also needs a reviewed order of operations. It should know the safe offline prefix, the gated suffix, exact stop conditions, and which evidence proves each step before it touches the terminal.

Implemented next: `rs autopilot runbook` writes `runbook.json` and `runbook.md`, refreshes `command-guard.json` / `.md`, and turns repeated commands, a command file, a run queue, or a fresh prompt into a step-by-step execution packet. The runbook wraps act-supported offline commands through `rs autopilot act`, separates live/apply commands into a gated suffix, records done-when and stop-if checks, and refuses to treat planned artifacts as completed evidence.

Why: This turns "the AI knows the next command" into "the AI knows the next safe command, the exact proof expected, and the moment it must stop." It is the missing layer between intent preflight and autonomous offline work.

## 95. Flight Recorder

A long AI build session should leave a black box. The next model should not infer from chat what happened; it should read one artifact that says which commands were recorded, which gates appeared, which evidence exists, which blockers remain, and which claims are allowed.

Implemented next: `rs autopilot flight-recorder` writes `flight-recorder.json` and `flight-recorder.md`. It reads run artifacts such as command guard, runbook, act, sprint, loop, journal, proof, acceptance, rehearsal, approval, live-gate, apply, rollback, and closeout, then extracts recorded commands, classifies them through command-guard rules, summarizes evidence events, counts blockers and claim guardrails, and recommends the next safe runbook, act, diagnose, or proof command.

Why: This makes context loss survivable. A fresh AI can resume from durable facts, not conversational fog, and it can distinguish "planned," "recorded," "gated," "executed," and "proved" before speaking to the creator.

## 96. Orientation Includes Safety Continuity

The new runbook and recorder only matter if a fresh model actually sees them. Cold-start orientation should refresh and require the safety-continuity artifacts for the selected run, not merely point at the older cockpit and timeline.

Implemented next: `rs autopilot orient` now refreshes `command-guard.json` / `.md`, `runbook.json` / `.md`, and `flight-recorder.json` / `.md` for the selected run. It carries their paths in `orientation.json`, marks runbook and flight recorder as required read-order items, and adds an operating rule telling the AI to use runbook safe prefixes and flight-recorder evidence before executing or reporting resumed work.

Why: This closes the handoff loop. A fresh or compacted AI gets tool knowledge, project habits, cockpit state, the safe command plan, and the black-box history in one first-read packet.

## 97. AI Navigator

Orientation tells an AI what to read; the next layer should tell it how to operate. A best-friend CLI should hand the model one card with the current situation, the first safe action, exact stop rules, and the claims it may and may not make.

Implemented next: `rs autopilot navigator` writes `navigator.json` and `navigator.md`. It refreshes orientation first, imports the required read order, runbook safe prefix, flight-recorder state, command guard status, proof and acceptance signals, safe-to-say and do-not-say claims, blockers, warnings, stop conditions, and artifact paths, then chooses one first safe action. It never executes work and tells the AI to refresh the navigator after one action.

Why: This gives a fresh model a cockpit card instead of a folder tour. It lowers cognitive load, makes one-step progress safer, and keeps claims tethered to proof while still letting the AI move.

## 98. One-Step Advance

Navigator gives the AI a first safe action. The next "best friend" move is letting the model execute that one action without copying shell text or accidentally crossing into live Studio.

Implemented next: `rs autopilot advance` writes `advance.json` and `advance.md`. It refreshes a before-state navigator, accepts only a navigator-selected `rs autopilot act <run> --command "<safe offline command>"` wrapper, executes the inner command through the existing `act` dispatcher, writes `act.json`, refreshes the after-state `navigator.json`, and stops. `--dry-run` selects the action without executing it.

Why: This makes safe progress feel real while preserving the safety contract. The AI can move one step, get a receipt, and then read the refreshed navigator before saying anything or taking another action.

## 99. Completion Audit

Even with proof, acceptance, fulfillment, and closeout packets, a model can still be tempted to summarize loosely. A best-friend CLI should hand it the exact checklist that answers, "Can I honestly call this request complete?"

Implemented next: `rs autopilot completion-audit` writes `completion-audit.json` and `completion-audit.md`. It refreshes closeout and fulfillment evidence, then maps the creator objective to concrete fulfillment items, closeout checks, safe-to-say claims, do-not-say claims, blockers, warnings, and exact next actions. It is also a safe `act` handler, so runbooks and navigators can route to it without shelling arbitrary commands.

Why: This gives an AI a final conscience artifact. It prevents "looks good" from becoming a false done claim and gives the next repair or live-proof command when the answer is not done yet.

## 100. Evidence-Based Delivery

After the AI knows whether work is done, it still needs to speak to the creator in a way that is helpful and evidence-bound. A final message should not be improvised from a folder scan or chat memory.

Implemented next: `rs autopilot deliver` writes `delivery.json` and `delivery.md`. It refreshes `completion-audit`, then emits the exact creator-facing message, first missing item, complete/offline/final status, safe claims, forbidden claims, artifacts, and next actions. It is also a safe `act` handler, so an AI can route to it from runbooks or navigator flows without arbitrary shell execution.

Why: This closes the loop from build evidence to communication. The AI can tell the creator what happened, what remains blocked, and what command comes next without accidentally upgrading offline work into live proof.

## 101. Model-Ready Context Pack

Orientation and navigator tell an AI what to read, but a context-compacted model still benefits from one bounded packet that carries the right snippets, the exact source links, and the next safe command. Without this, a resumed AI wastes attention reopening every artifact or, worse, trusts stale chat summaries.

Implemented next: `rs autopilot model-pack` writes `model-pack.json` and `model-pack.md`. It refreshes navigator and delivery evidence, embeds redacted bounded snippets from high-value artifacts, preserves source paths as the evidence of record, writes a resume prompt, carries safe claims and forbidden claims, and is available as a safe `act` handler. The `--max-chars` budget keeps the pack loadable while marking omitted snippets explicitly.

Why: This is the compact handoff artifact a model actually wants in its context window. It makes resume faster, safer, and less dependent on chat scrollback while preserving the rule that source artifacts and completion-audit remain authoritative.

## 102. Agent Task Pack

Once a model has trustworthy context, the next bottleneck is turning that context into one disciplined task for a fresh coding agent. The AI needs a packet that says what to do, what command is allowed, what validation must run, what artifacts prove progress, and exactly when to stop.

Implemented next: `rs autopilot task-pack` writes `task-pack.json` and `task-pack.md`. It refreshes model-pack, opportunities, and work-order evidence, then emits a copy/paste-ready task prompt with the selected opportunity, primary command, allowed commands, validation commands, expected artifacts, acceptance checks, stop conditions, safe claims, forbidden claims, blockers, warnings, and source links. It is also a safe `act` handler.

Why: This is the handoff artifact that turns "here is the state" into "here is your next safe job." It lets a fresh agent move quickly without inventing commands, weakening proof rules, or mistaking instructions for completed work.

## 103. Parallel Agent Squad Pack

As the work becomes more ambitious, one AI agent is not always enough. A best-friend CLI should help the lead model split a run into several safe, non-overlapping assignments so multiple agents can work in parallel without guessing ownership, commands, validation, or stop rules.

Implemented next: `rs autopilot squad-pack` writes `squad-pack.json` and `squad-pack.md`. It refreshes model-pack and opportunities, then emits a coordination prompt plus per-agent assignments with roles, ownership boundaries, objectives, primary commands, allowed commands, validation commands, expected artifacts, stop conditions, safe claims, forbidden claims, blockers, warnings, and source links. It is also a safe `act` handler.

Why: This is the product becoming a team lead for AI builders. It lets one model hand out focused work to several agents while keeping proof, live mutation, and creator-facing claims under the same guardrails.

## 104. Parallel Agent Integration Review

Splitting work is only half of collaboration. A lead AI also needs to reconcile what the parallel agents produced, whether their evidence exists, whether journal continuity is present, and whether two agents were assigned the same artifact surface.

Implemented next: `rs autopilot squad-review` writes `squad-review.json` and `squad-review.md`. It refreshes the squad pack, reviews each assignment's expected artifacts and journal evidence, detects duplicate artifact ownership, emits an integration prompt, marks assignment review status, carries safe and forbidden claims, and recommends exact next actions. It is also a safe `act` handler.

Why: This gives the CLI the other side of multi-agent work: not just "divide the jobs," but "bring the jobs back together honestly." The lead model can see which lanes are complete, which need evidence, and which conflicts must be resolved before any creator-facing update.

## 105. Wow Factor Planning

Correct generated slices can still feel flat. A best-friend CLI should help the AI choose one memorable Roblox player moment before more agents spend effort on ordinary implementation work.

Implemented next: `rs autopilot wow-plan` writes `wow-plan.json` and `wow-plan.md`. It reads the prompt plus run artifacts, derives current creative signals, ranks safe wow-factor candidates, selects one player moment, lists proof needs, carries safe and forbidden claims, and recommends exact non-mutating next commands. It is also a safe `act` handler.

Why: This gives the product a taste layer. The AI can now ask "what makes this feel special?" from artifacts, then build and prove that candidate without pretending a plan is already an implemented feature.

## 106. Wow Moment Implementation Pack

Choosing the wow factor is still not enough. A fresh AI agent needs the exact candidate run, commands, proof checklist, stop conditions, and claim boundaries that turn a good taste call into buildable work.

Implemented next: `rs autopilot moment-pack` writes `moment-pack.json` and `moment-pack.md`. It refreshes `wow-plan`, selects the requested or top idea, creates implementation lanes for candidate generation, experience proof, and claim-safe handoff, then emits validation commands, a bounded task prompt, safe claims, forbidden claims, blockers, warnings, and exact next actions. It is also a safe `act` handler.

Why: This turns the product's creative taste into execution. The AI can now move from "this is the hook" to "here is the safe implementation job" without inventing commands or mistaking the brief for proof.

## 107. Offline Wow Candidate Sprint

An AI's best friend should not stop at a build brief. Once the moment is selected and scoped, the CLI should be able to produce a separate offline candidate and refresh the evidence around it without asking the model to manually copy commands.

Implemented next: `rs autopilot moment-sprint` writes `moment-sprint.json` and `moment-sprint.md`. It refreshes `moment-pack`, generates the selected wow moment as a separate candidate run, then refreshes showcase, gameplay critique, proof, acceptance, completion-audit, and claim-check artifacts for that candidate. It records every step, artifact, warning, blocker, safe claim, forbidden claim, and next action, and it is available as a safe `act` handler.

Why: This is the first taste-to-artifact loop. The AI can now choose a hook, package it, generate the candidate, and inspect review evidence while still staying outside Studio mutation and live-proof claims.

## 108. Offline Wow Continuation Decision

Building a wow candidate is not enough. A best-friend CLI should also decide whether that candidate is actually the stronger continuation, refresh the recommended review evidence, and tell the AI what it may safely show the creator next.

Implemented next: `rs autopilot moment-decision` writes `moment-decision.json` and `moment-decision.md`. It refreshes `moment-sprint`, compares the generated candidate against the source run, refreshes the recommended run's review pack and claim check, then records the decision, evidence, blockers, warnings, safe claims, forbidden claims, next actions, and agent brief. It is also available as a safe `act` handler.

Why: This closes the taste loop from "I built a wow candidate" to "this is the proof-bound run I should carry forward." The AI can now choose the best offline continuation without pretending it has creator approval, live apply, playtest, rollback, publish, or production proof.

## 109. Proof-Bound Creator Demo Packet

Once the AI knows the best wow continuation, it needs one artifact for showing the creator. A best-friend CLI should assemble the talk track, proof table, approval boundary, checked message, and demo runbook so the model does not improvise beyond evidence.

Implemented next: `rs autopilot creator-demo` writes `creator-demo.json` and `creator-demo.md`. It refreshes `moment-decision`, then refreshes the recommended run's showcase, review pack, delivery, and rehearsal artifacts. The packet records a creator message, talk track, proof table, approval boundary, safe claims, forbidden claims, blockers, warnings, next actions, and agent brief. It is also available as a safe `act` handler.

Why: This turns a chosen candidate into a creator-ready presentation while preserving the line between "ready to review" and "approved, applied, playtested, or production-ready." The AI gets a script and proof map instead of relying on chat memory or overselling.

## 110. Post-Demo Response Router

The demo is only useful if the AI knows what to do with the creator's reaction. A best-friend CLI should turn "ship it," "make the button brighter," "not this direction," or a follow-up question into the right safe artifact path without treating chat text as live approval.

Implemented next: `rs autopilot demo-response` writes `demo-response.json` and `demo-response.md`. It refreshes `creator-demo`, classifies the creator response, then routes tweaks into `feedback.json`, `feedback-patch.json`, and `feedback-planner-pack.json`; approval-like wording into `approval.json` and live-gate next actions; redirection into `decisions.json`; and question-like replies into checked-response handling. It is also available as a safe `act` handler.

Why: This gives the AI a real conversation loop after the wow demo. The creator can react naturally, and the CLI turns that reaction into the next safe move instead of relying on the model to remember which approval, feedback, or claim boundary applies.

## 111. Post-Demo Continuation Handoff

Routing is useful, but the next AI still needs the actual work packet. A best-friend CLI should turn the routed reaction into a model-ready prompt, command queue, expected artifacts, and stop conditions so a fresh session can continue without rereading every prior JSON file.

Implemented next: `rs autopilot demo-loop` writes `demo-loop.json` and `demo-loop.md`. It refreshes `demo-response`, embeds the route, then writes one handoff prompt with the feedback planner prompt, approval gate script, wow redirection path, or checked-response path as appropriate. It records the command queue, expected artifacts, stop conditions, safe claims, forbidden claims, blockers, warnings, and agent brief. It is also available as a safe `act` handler.

Why: This turns creator reaction into momentum. After a demo, the AI no longer has to infer whether it should patch, ask for approval, pivot, or answer; the CLI hands it the exact next packet and the stop rules.

## 112. Post-Demo Follow-Through Check

The handoff is only trustworthy if the AI can tell whether the follow-up actually happened. A best-friend CLI should audit the routed reaction before any creator-facing claim, separating "ready for next AI" from "feedback implemented," "approval prepared," or "response safe to send."

Implemented next: `rs autopilot demo-check` writes `demo-check.json` and `demo-check.md`. It reads `demo-loop` and `demo-response`, checks route-specific artifacts, marks feedback routes as needing follow-up until an adopted patch run exists, treats approval as prepared without crossing live apply, and records safe claims, forbidden claims, blockers, warnings, next actions, and an agent brief. It is also available as a safe `act` handler.

Why: This closes the post-demo honesty loop. The AI can keep momentum after a creator reaction while still knowing exactly what it can and cannot say.

## 113. Checked Post-Demo Creator Reply

The AI should not have to improvise the final message after a demo check. A best-friend CLI should turn the checked route state into exact creator-facing wording, then claim-check that wording so feedback, approval, redirection, and answer routes stay honest.

Implemented next: `rs autopilot demo-reply` writes `demo-reply.json` and `demo-reply.md`. It refreshes `demo-check`, writes `demo-reply-claim-check.json`, composes the exact creator message, preserves route status, checked claims, safe claims, forbidden claims, blockers, warnings, next actions, and an agent brief, and is available as a safe `act` handler.

Why: This completes the post-demo conversation loop. The AI can now show a wow candidate, route the creator reaction, package the next work, audit follow-through, and send a safe update without inventing completion or live proof.

## 114. Post-Demo Learning Packet

A best-friend AI should learn from the creator's reaction instead of treating every demo as an isolated chat turn. After the safe reply, the CLI should distill taste, constraints, follow-up themes, and memory actions that future agents can use without claiming the learning was already persisted.

Implemented next: `rs autopilot demo-learn` writes `demo-learn.json` and `demo-learn.md`. It refreshes `demo-reply`, reads the routed creator messages, extracts creator signals, learned preferences, constraints, follow-up themes, recommended memory actions, safe claims, forbidden claims, next actions, and an agent brief, and is available as a safe `act` handler.

Why: This gives the AI continuity of taste. The next session can preserve what the creator liked or wanted changed, while still routing durable updates through `preferences`, `memory`, `game-bible`, `playbook`, or `decisions`.

## 115. Post-Demo Memory Consolidation

Learning is only useful if the next AI actually loads it. A best-friend CLI should turn a demo reaction into the refreshed project memory, creator preferences, game bible, and operating playbook in one safe step.

Implemented next: `rs autopilot remember` writes `remember.json` and `remember.md`. It refreshes `demo-learn`, regenerates project memory, creator preferences, game bible, and AI playbook artifacts, then records consolidated artifact statuses, memory actions, safe claims, forbidden claims, next actions, and an agent brief. It is also available as a safe `act` handler.

Why: This closes the taste-to-context loop. After a creator reacts to a demo, a future AI can orient from refreshed durable context instead of manually remembering which four project-level commands to run.

## 116. AI Best-Friend Launch Packet

Context still has to become behavior. A fresh AI should receive one launch packet that says what it knows about the creator, which artifacts to read, what task is safe, where to stop, and what claims are allowed.

Implemented next: `rs autopilot best-friend` writes `best-friend.json` and `best-friend.md`. It refreshes `remember`, builds the nested model pack and task pack, then emits an opening prompt, companion contract, context cards, required read order, first safe action, safe claims, forbidden claims, next actions, and an agent brief. It is also available as a safe `act` handler.

Why: This is the product's "start here" surface for AI agents. Instead of asking the model to stitch together memory, orientation, model context, and work orders, the CLI gives it one evidence-bound companion packet for helping the creator build.

## 117. AI Self-Check Before Speaking Or Acting

Even with a strong launch packet, an AI can still overclaim in its next message or copy a live command too early. A best-friend CLI should give the model one last preflight that checks proposed wording and proposed commands together.

Implemented next: `rs autopilot self-check` writes `self-check.json` and `self-check.md`. It combines claim-check and command-guard evidence, accepts proposed `--claim`, `--message`, `--command`, and `--from-file` inputs, writes child self-check claim/command artifacts, and returns `readyToProceed`, `needsRewriteOrGate`, `blocked`, or `needsInput`. It is also available as a safe `act` handler.

Why: This turns safety from a remembered rule into a reflex. Before the AI speaks or acts, it can ask the CLI whether the wording is evidence-backed and whether the commands stay offline.

## 118. Self-Protecting Launch Packet

A fresh AI should not have to remember to run a separate preflight before using the launch packet. The best-friend packet should include its own checked first action and checked opening claim so the handoff is both helpful and immediately safe.

Implemented next: `rs autopilot best-friend` now writes `best-friend-self-check.json` and `best-friend-self-check.md`. It uses the new self-check path to verify the task-pack launch claim and first safe action, adds the preflight as a context card and artifact, includes the status in the Markdown, and blocks the launch packet if that preflight is not safe to proceed.

Why: This makes the product feel like a real AI operating system instead of a folder of suggestions. A new model receives context, task instructions, stop rules, and a preflighted first move in one artifact-backed handoff.

## 119. Checked Opening Reply

Even a self-protecting launch packet can leave the model improvising its first creator-facing sentence. A best-friend CLI should provide exact opening wording that explains the checked claim, the next offline move, and the stop boundary without pretending work has already run.

Implemented next: `rs autopilot best-friend` now includes `openingReply` and `openingReplyClaim` in `best-friend.json`, plus an "Opening Reply" section in `best-friend.md`. The reply is built from the launch self-check claim and first safe action, so a fresh AI can greet the creator with safe wording before taking the next action.

Why: This closes the handoff-to-human gap. The model gets not only instructions for itself, but also the first safe sentence it can say to the creator.

## 120. Protected First Action

A checked first action is safer when the AI does not have to run the raw command directly. If the command is supported by `act`, the launch packet should hand the model the protected `rs autopilot act ... --command ...` wrapper as the preferred execution path.

Implemented next: `rs autopilot best-friend` now emits `firstActAction` when the first safe action is dispatchable by `act`, makes that protected command the first next action, and includes the protected command in the opening prompt, opening reply, summary output, and Markdown. `self-check` also prefers an `act` wrapper for ready offline commands.

Why: This makes the first move safer and more automatic. A fresh AI can greet the creator, then execute exactly one checked offline action through the internal dispatcher instead of hand-running arbitrary shell text.

## 121. First-Turn Receipt

A protected first command is still only a recommendation until the AI has a single safe way to take it and preserve the evidence. The product should let a fresh model make exactly one checked offline move, record the nested act result, and refresh the launch packet before it says or does anything else.

Implemented next: `rs autopilot first-turn` now writes `first-turn.json` and `first-turn.md`. It snapshots `first-turn-best-friend-before.json`, executes the best-friend protected action through `act` as `first-turn-act.json`, refreshes `best-friend.json`, and reports safe-to-say, do-not-say, blockers, warnings, artifacts, and next actions. A `--dry-run` mode selects the protected first action without executing it.

Why: This turns the handoff from a smart packet into an actual first move. The AI gets one bounded step forward, plus a receipt that prevents it from claiming broader completion, live Studio mutation, publish, or playtest evidence.

## 122. Bounded Best-Friend Loop

One safe turn is strong, but a helpful AI often needs a tiny operating loop: check the refreshed packet, take one protected action, stop if the next action repeats, and leave a receipt trail. The product should make that pattern explicit so the model does not drift into unbounded autonomy.

Implemented next: `rs autopilot best-friend-loop` now writes `best-friend-loop.json` and `best-friend-loop.md`. It previews each `best-friend` turn, executes non-repeated protected actions through `first-turn`, writes per-turn receipts such as `best-friend-loop-turn-01.json` and `best-friend-loop-turn-01-act.json`, refreshes `best-friend.json`, and stops at repeats, blockers, dry-run, or the step limit.

Why: This gives an AI model a small, disciplined work rhythm. It can make bounded offline progress for the creator while preserving every receipt and refusing the two classic failure modes: repeating the same command forever or crossing a live boundary.

## 123. Checked Best-Friend Reply

After an AI acts, the next risk is not code; it is wording. The model needs a creator-facing update that says exactly what the receipts prove, names the live boundary, and refuses completion claims unless separate evidence supports them.

Implemented next: `rs autopilot best-friend-reply` now writes `best-friend-reply.json` and `best-friend-reply.md`. It reads `best-friend-loop.json`, `first-turn.json`, or `best-friend.json`, drafts the status message, runs the claim and message through `self-check` as `best-friend-reply-self-check.json`, and returns `readyToSend`, `needsRewrite`, or `blocked`.

Why: This closes the act-to-speak gap. A best-friend AI should not improvise after it makes progress; it should hand the creator wording that has already been checked against the actual artifact trail.

## 124. One-Turn Best-Friend Operator

Individual launch, loop, and reply commands are powerful, but the AI still benefits from a single top-level receipt for one ordinary operating turn. A best-friend CLI should safely handle the latest creator message, run a bounded protected action loop, and prepare the checked creator-facing wording without making the model stitch receipts together from memory.

Implemented next: `rs autopilot best-friend-turn` writes `best-friend-turn.json` and `best-friend-turn.md`. It optionally routes `--message` through `chat`, runs `best-friend-loop`, drafts `best-friend-reply`, records each child step, preserves approval/live gates, and returns the exact checked best-friend message only when it is ready to send.

Why: This makes the product feel like a real AI companion loop. The model gets one command for "listen, safely act, then speak honestly," while every live mutation, publish, upload, and playtest claim remains gated by evidence.

## 125. First-Contact Best-Friend Session

A creator should not have to know whether the AI needs `session`, `start`, `chat`, or `best-friend-turn`. A best-friend CLI should accept the first creator request, bootstrap a safe offline run when needed, resume a known run when one is provided, and finish with a checked companion message.

Implemented next: `rs autopilot best-friend-session` writes `best-friend-session.json` and `best-friend-session.md`. It bootstraps through `session` when no existing run is available, resumes directly when `--run-dir` points at a run, runs `best-friend-turn`, preserves every child artifact, and exposes `bestFriendMessage` only when the nested reply is ready to send.

Why: This turns the product from a toolkit into a first-contact companion. A model can meet the creator cold, create or resume the right offline evidence surface, take one protected action, and answer honestly without guessing the command sequence.

## 126. First-Contact Wow Demo Session

The real creator-facing magic is not only a safe reply; it is arriving at a concrete player moment that the creator can review. A best-friend CLI should turn a cold request or resumed run into the full offline wow-demo chain without making the model remember the sequence.

Implemented next: `rs autopilot wow-session` writes `wow-session.json` and `wow-session.md`. It bootstraps or resumes through `best-friend-session`, then prepares the proof-bound creator demo through `creator-demo`, preserving the selected idea, recommended run, demo title, talk-track evidence, safe claims, forbidden claims, blockers, warnings, and next actions.

Why: This is the shortest path from "I want to build a Roblox game" to "here is the memorable offline demo moment and the evidence boundary." The AI can meet the creator, generate the candidate, prepare the demo, and still refuse live apply, playtest, publish, or approval claims until the proper gates exist.

## 127. Post-Demo Companion Session

After the demo, the AI's next job is delicate: route the creator's reaction, verify what was actually handled, speak honestly, and remember the taste signal. A best-friend CLI should make that entire reaction loop one safe receipt instead of a chain the model stitches together from memory.

Implemented next: `rs autopilot demo-session` writes `demo-session.json` and `demo-session.md`. It refreshes `demo-loop`, audits route-specific evidence through `demo-check`, drafts checked wording through `demo-reply`, distills reusable signals through `demo-learn`, consolidates memory through `remember`, and returns `replyMessage` only when the nested reply is ready to send.

Why: This makes the product feel like a real creative partner after the first wow moment. The AI can hear "looks good, but make the shop brighter," preserve it as feedback and preference, tell the creator exactly what is and is not done, and carry the learning into future sessions without pretending it applied anything live.

## 128. Whole Creator Arc Receipt

The strongest companion surface is not a chain of commands; it is one durable receipt that says where the creator is in the journey. A best-friend CLI should prepare the first wow demo, then accept the creator's reaction and leave the next AI with the checked reply, learned preference, proof boundary, and continuation command in one place.

Implemented next: `rs autopilot best-friend-arc` writes `best-friend-arc.json` and `best-friend-arc.md`. Without `--message`, it refreshes `wow-session` and becomes a demo-ready read-first packet. With `--message`, it also runs `demo-session`, returns `replyMessage` only when checked, refreshes memory, and records safe claims, forbidden claims, blockers, warnings, artifacts, and next actions.

Why: This is the "AI best friend" command a fresh model can reach for when it does not know which part of the demo loop the creator is in. The same command can start the proof-bound wow moment or handle the reaction afterward without crossing live apply, publish, upload, playtest, rollback, or production proof boundaries.

## 129. Studio-Visible Best-Friend Arc

The arc should not live only in terminal output. A creator reviewing inside Roblox Studio should see the same selected wow moment, proof boundary, checked demo message, checked post-demo reply, and next safe action that the AI sees in `best-friend-arc.json`.

Implemented next: `rs autopilot publish-review` now accepts `--arc <best-friend-arc.json>` and auto-detects `<run-dir>/best-friend-arc.json` when present. The Studio review payload includes arc status, phase, verdict, selected wow moment, demo title, creator demo message, checked reply, ready flags, and recommended run path. The `AutopilotReview` plugin panel renders an "AI companion arc" section when those fields are present.

Why: This closes the loop between CLI intelligence and creator trust. The AI can prepare a proof-bound moment, route feedback honestly, and then publish the exact same safe state into Studio without making the review panel an approval, apply, publish, upload, playtest, rollback, or production-proof gate.

## 130. Studio-Visible AI Launch Context

The panel should also help the next AI model, not only the human reviewer. When an AI is operating from Studio context, it should see the same `best-friend.json` read order, checked opening reply, first safe action, protected `act` wrapper, and stop rules that the terminal packet provides.

Implemented next: `rs autopilot publish-review` now accepts `--best-friend <best-friend.json>` and auto-detects `<run-dir>/best-friend.json` when present. The Studio review payload includes best-friend status, relationship mode, task-pack status, launch preflight status, checked opening reply, opening claim, first safe action, protected first action, read order, context cards, companion contract, and artifact path. The `AutopilotReview` plugin panel renders an "AI best friend context" section when those fields are present.

Why: This turns Studio into a shared AI cockpit. A creator can inspect what the next AI is about to say and do, while the AI can resume from the same bounded context without executing the action, approving apply, mutating Studio, or inventing proof.

## 131. Best-Friend Default After Remember

Memory consolidation is the hinge between "we learned something" and "the next AI can act on it." After `remember` refreshes project memory, creator preferences, game bible, and AI playbook, the model should not have to infer the next launch command from docs or chat history.

Implemented next: `rs autopilot remember` now makes `rs autopilot best-friend <run-dir> --root <root> --limit <n> --format json` the first normal next action. The remember packet and Markdown still include orientation and playbook refresh commands, but the default continuation now packages remembered context into a checked best-friend launch surface with read order, opening reply, first safe action, protected `act` wrapper, and stop boundaries.

Why: This closes the memory-to-action gap. A context-compacted or newly spawned AI can refresh durable learning, then immediately receive the product's strongest "start here" companion packet without guessing the command sequence or claiming memory consolidation implemented anything in Studio.

## 132. One-Command Memory-To-Launch Packet

Recommendation is helpful, but sometimes the AI needs one receipt that both refreshes durable learning and writes the next launch packet. The implementation must avoid a naive `remember -> best-friend -> remember` loop while keeping memory consolidation non-mutating.

Implemented next: `rs autopilot remember` now accepts `--best-friend`, plus optional `--opportunity` and `--max-chars`. When enabled, it builds `best-friend.json` and optional Markdown from the just-refreshed `remember` report, records `bestFriendStatus`, `bestFriendPath`, `bestFriendMarkdownPath`, and a `bestFriend` consolidated artifact in `remember.json`, and promotes the generated best-friend packet's first protected action into the remember next-action list. The command still only writes offline packets; it does not execute the first action, mutate Studio, approve apply, publish, upload, or prove live playtests.

Why: This turns post-demo learning into immediate AI readiness. A model can run one command after a creator reaction and leave behind both durable project memory and the exact checked packet a fresh model should read before speaking or acting.

## 133. Best-Friend Launch Control

A launch packet is powerful, but a fresh AI also needs a final "am I truly ready to operate?" checklist before it speaks or runs the protected action. The check should audit the launch packet, memory, read order, self-check, protected first action, and Studio review visibility without crossing any execution boundary.

Implemented next: `rs autopilot best-friend-check` writes `best-friend-check.json` and `best-friend-check.md`. It refreshes `best-friend.json`, audits required memory artifacts, required read order, launch preflight, offline/non-mutating first action, protected `act` wrapper, checked opening reply, source artifact links, and optional Studio panel visibility. The receipt includes a launch-control prompt, checklist, exact repair commands, safe claims, forbidden claims, blockers, warnings, and next actions. It is also a safe `act` handler, but it still only audits launch readiness; it does not execute the protected first action, mutate Studio, publish, upload, prove live playtests, or mark work complete.

Why: This gives a context-compacted model the confidence check it wants before moving. It can tell the difference between "ready to start with the protected action" and "repair the handoff first," instead of improvising from a large pile of JSON.

## 134. Best-Friend Rescue Mode

Even with launch control, an AI companion will sometimes hit a failed command, stale bundle, blocked check, missing evidence, or confusing live gate. The model needs one recovery receipt that says what failed, what is safe to say, what not to claim, and which repair command to try next without pretending the repair already happened.

Implemented next: `rs autopilot best-friend-rescue` writes `best-friend-rescue.json` and `best-friend-rescue.md`. It refreshes `best-friend-check`, optionally writes `best-friend-rescue-diagnosis.json` from supplied `--command`, `--result`, `--error`, and `--evidence` inputs, selects the safest repair action, and checks the recovery message through `best-friend-rescue-self-check.json`. It is also a safe `act` handler. The command does not execute the selected repair, mutate Studio, publish, upload, prove live playtests, or mark work complete.

Why: This is the companion's panic button. A fresh or stuck model can recover from failure with artifact-backed guidance and honest wording instead of guessing, retrying arbitrary commands, or telling the creator a blocker was fixed.

## 135. Best-Friend Mentor Mode

Need: A fresh AI needs more than artifacts and commands. It needs a small coaching surface that explains what to read first, why the next command matters, which mistake would break trust, and when to stop and recover.

Implemented next: `rs autopilot best-friend-mentor` writes `best-friend-mentor.json` and `best-friend-mentor.md`. It refreshes `best-friend-check`, converts launch-control evidence into coaching cards, carries required read order, records mistake traps, chooses the protected next action or `best-friend-rescue` when launch control is blocked, and is available as a safe `act` handler. It does not execute the selected action, mutate Studio, publish, upload, prove live playtests, or mark work complete.

Why: This turns the CLI from a receipt generator into a coach for the model itself. A resumed AI can quickly understand the run, avoid the most tempting unsafe shortcut, and continue like a careful teammate instead of a stateless command runner.

## 136. Best-Friend Pilot Mode

Need: Once the AI has launch control and mentoring, the next leap is a single self-healing companion move. The model should not have to manually stitch together mentor, first-turn, reply, and rescue commands just to take one safe step.

Implemented next: `rs autopilot best-friend-pilot` writes `best-friend-pilot.json` and `best-friend-pilot.md`. It refreshes `best-friend-mentor`, runs exactly one protected offline `first-turn` when launch control is ready, prepares `best-friend-reply` with a checked `companionMessage`, or writes `best-friend-rescue` when launch control is blocked. It is available as a safe `act` handler and supports `--dry-run`. It does not run a second action, mutate Studio, apply, publish, upload, prove live playtests, or mark work complete.

Why: This is the first true co-pilot loop. A fresh model can safely coach itself, move the project forward by one bounded offline action, and speak honestly about the result from one receipt.

## 137. Studio Review Co-Pilot Visibility

Need: Once the pilot can safely move and reply, the creator needs to inspect that co-pilot result inside Studio instead of hunting for a CLI artifact.

Implemented next: `rs autopilot publish-review` now accepts `--best-friend-pilot <best-friend-pilot.json>` and auto-detects `<run-dir>/best-friend-pilot.json` when present. The Studio review payload includes co-pilot status, mode, verdict, mentor/first-turn/rescue/reply statuses, protected-action readiness, selected command, checked companion message, and pilot next actions. The `AutopilotReview` plugin panel renders an "AI co-pilot" section when those fields are present.

Why: This makes the wow factor visible where the creator works. The AI can show its latest safe move, the exact words it is allowed to say, and the live-mutation boundary without pretending the Studio place was changed.

## 138. Best-Friend Control Router

Need: A context-compacted or newly resumed AI still needs one authoritative "what now?" receipt after launch-control, pilot, rescue, reply, or Studio-review changes. Without that router, the model has to infer whether to act, speak, publish, or recover from scattered artifacts.

Implemented next: `rs autopilot best-friend-control` writes `best-friend-control.json` and `best-friend-control.md`. It refreshes `best-friend-check`, reads the latest `best-friend-pilot.json` and `studio-review.json`, then selects exactly one branch: run the pilot, run rescue, publish the pilot to Studio, refresh a checked reply, or send the checked companion message. The receipt includes an operator prompt, ordered steps, selected command, safe claims, forbidden claims, and the companion message only when the pilot is ready and visible.

Why: This gives the AI a control tower. It can resume from one artifact, avoid guessing from stale chat context, and choose the right next move without accidentally doing a second action or overstating Studio proof.

## 139. Best-Friend Operate Step

Need: Control can choose the next branch, but the AI still has to copy the selected command, run it, and remember to refresh control. That is exactly where context-compacted models make double-action or overclaim mistakes.

Implemented next: `rs autopilot best-friend-operate` writes `best-friend-operate.json` and `best-friend-operate.md`. It first writes `best-friend-operate-control-before.json`, executes exactly one selected offline branch (`best-friend-pilot`, `best-friend-rescue`, or `best-friend-reply`), writes `best-friend-operate-control-after.json`, and returns the updated next boundary. If control selects `publishPilot`, it stops with the exact `publish-review` command instead of touching Studio. If control selects `speak`, it returns the checked companion message without executing another branch.

Why: This is the safe operator hand. A model can ask for one next move, let the CLI perform only the allowed offline branch, and receive a fresh control state before it thinks about doing anything else.

## 140. Best-Friend Runner Supervisor

Need: One-step operation is safe, but it still leaves a model manually repeating the same command until something meaningful stops it. A true AI companion needs a bounded supervisor that can keep advancing safe offline work while preserving every receipt and stopping at real boundaries.

Implemented next: `rs autopilot best-friend-runner` writes `best-friend-runner.json` and `best-friend-runner.md`. It runs bounded `best-friend-operate` steps, writes per-step receipts such as `best-friend-runner-step-01.json`, preserves prefixed control-before/control-after receipts, and stops at checked speech, Studio publish, blockers, dry-run, repeated branch risk, or `--max-steps`.

Why: This gives the model an autopilot-within-Autopilot loop that still respects the live boundary. It can make progress without command-copying, yet it cannot publish, apply, upload, prove playtests, claim completion, or repeat a branch without fresh review.

## 141. Studio Review Runner Visibility

Need: The runner is the most complete AI companion receipt, but the creator should not need to inspect CLI JSON to understand what happened. The Studio review panel needs to show the bounded supervisor state alongside the co-pilot and launch context.

Implemented next: `rs autopilot publish-review` accepts `--best-friend-runner <best-friend-runner.json>` and auto-detects `<run-dir>/best-friend-runner.json` when present. The Studio payload now includes runner status, verdict, stop reason, executed/max steps, last branch, checked companion message, runner steps, and next actions. The `AutopilotReview` plugin renders an "AI runner" section.

Why: This closes the visibility gap for the product's "wow" loop. The AI can run bounded offline work, publish the proof-bound state to Studio, and let the creator inspect exactly why it stopped before any live boundary.

## Implementation Priority

1. Ship deterministic Autopilot planning, preview, apply, and report.
2. Add reusable context bundle generation.
3. Add more game feature recipes.
4. Add recipe composition for complete starter-game slices.
5. Add prompt-to-architecture planning for large AI-led builds.
6. Add provider-ready planning packets for model-backed plans.
7. Add strict model output intake.
8. Add project memory ledgers.
9. Add gameplay critique loops before live apply.
10. Add autonomous offline improvement loops.
11. Add multi-step offline iteration loops.
12. Expand richer rollback restore and Studio-side review surfaces.
13. Harden rollback restore.
14. Add the Studio review panel.
15. Add rollback readiness packets that prevent unsupported undo claims.
16. Add approval packets that make the mutation boundary explicit for AI agents and creators.
17. Add privacy scans that make AI handoff safe by default.
18. Fence destructive `deleteInstance` plans behind danger risk, force, and rollback capture.
19. Add an applied-health gate that makes regression smoke mandatory before healthy-run claims.
18. Add AI continuation capsules that make cross-session handoff copy/paste-safe.
19. Add agenda sprints that execute bounded safe agenda actions and stop at live gates.
20. Add AI work retrospectives that convert run evidence into lessons and next commands.
21. Add AI operating playbooks that convert project memory, canon, preferences, and retrospectives into reusable work habits.
22. Add capability atlases that teach AI agents the verified command, recipe, artifact, and safety surface before they act.
23. Make cold-start orientation tool-aware by refreshing the capability atlas and playbook as required read-order artifacts.
24. Add command sequence guards that preflight AI-proposed commands before `act`, manual execution, or live-boundary work.
25. Add execution runbooks that split guarded queues into a safe offline prefix, gated suffix, stop conditions, and expected evidence.
26. Add flight recorders that preserve command, gate, evidence, blocker, and claim history for AI resume.
27. Make orientation refresh and require runbook plus flight-recorder safety-continuity artifacts for selected runs.
28. Add AI navigator cards that condense orientation, runbook, recorder, proof, stop rules, and one first safe action.
29. Add one-step advance receipts that execute exactly one navigator-selected safe `act` action and refresh the card.
30. Add completion audits that map the creator objective to concrete evidence before any final done claim.
31. Add evidence-based delivery packets that turn completion audits into exact creator-facing messages.
32. Add model-ready context packs that embed bounded redacted snippets and source links for context-compacted AI resume.
33. Add agent task packs that turn model context and work orders into one copy/paste-ready safe task.
34. Add squad packs that split opportunity maps into parallel agent assignments with ownership and stop rules.
35. Add squad reviews that reconcile parallel agent evidence, journal continuity, and ownership conflicts.
36. Add wow plans that rank memorable player moments, proof needs, and safe next commands before more implementation work.
37. Add moment packs that convert selected wow ideas into implementation lanes, validation commands, and stop conditions.
38. Add moment sprints that generate and review a separate offline wow candidate run.
39. Add moment decisions that compare the wow candidate with the source run and recommend the proof-bound continuation.
40. Add creator demo packets that present the recommended wow run with talk track, proof table, and approval boundary.
41. Add post-demo response routing that turns creator reactions into feedback, approval, decision, or checked-response artifacts.
42. Add post-demo continuation handoffs that package the routed reaction into an AI prompt, command queue, expected artifacts, and stop rules.
43. Add post-demo follow-through checks that audit route-specific evidence before creator-facing claims.
44. Add checked post-demo replies that turn demo-check state into safe creator-facing wording.
45. Add post-demo learning packets that distill creator reactions into reusable taste and constraint signals.
46. Add post-demo memory consolidation so demo learning refreshes project memory, preferences, canon, and playbook context in one safe step.
47. Add AI best-friend launch packets that combine remembered context, model context, task instructions, read order, and one first safe action.
48. Add AI self-check packets that combine claim-check and command-guard before the model speaks or acts.
49. Make best-friend launch packets self-protecting by embedding a launch self-check for the first safe action.
50. Add checked opening replies so fresh AI sessions can speak safely before acting.
51. Prefer protected `act` wrappers for checked first actions.
52. Add first-turn receipts that execute exactly one protected best-friend action and refresh the launch packet.
53. Add bounded best-friend loops that execute non-repeated protected turns and stop at safety boundaries.
54. Add checked best-friend replies that turn receipts into safe creator-facing updates.
55. Add one-turn best-friend operators that route a creator message, run protected work, and return a checked reply as one receipt.
56. Add first-contact best-friend sessions that bootstrap or resume the run before the protected turn.
57. Add first-contact wow demo sessions that bootstrap or resume the run before preparing the proof-bound creator demo.
58. Add post-demo companion sessions that route reactions, check replies, learn preferences, and refresh remembered context.
59. Add whole creator arc receipts that prepare the proof-bound demo and optionally handle the post-demo reaction as one read-first artifact.
60. Make Studio review publishing arc-aware so creators can inspect the selected wow moment and checked reply inside Studio.
61. Make Studio review publishing best-friend-aware so creators and resumed AI sessions can inspect the launch context, checked opening reply, and protected first action inside Studio.
62. Make `remember` recommend `best-friend` as the default continuation so refreshed creator learning turns into a checked AI launch packet.
63. Add `remember --best-friend` so one non-mutating command refreshes durable learning and writes the checked AI launch packet without recursive memory refresh.
64. Add `best-friend-check` launch control so a fresh AI can audit packet, memory, read order, preflight, protected action, and Studio review visibility before speaking or acting.
65. Add `best-friend-rescue` so stuck AI companion sessions get launch-control, diagnosis, selected repair commands, and checked recovery wording without pretending the repair ran.
66. Add `best-friend-mentor` so fresh or resumed AI sessions get read order, why-this-next-move guidance, mistake traps, and a rescue-vs-action decision before speaking or acting.
67. Add `best-friend-pilot` so a model can mentor itself, execute exactly one protected offline action, or prepare rescue, then return checked creator-facing wording from one receipt.
68. Make Studio review publishing best-friend-pilot-aware so creators can inspect the co-pilot status, checked message, and selected protected command inside Studio.
69. Add `best-friend-control` so resumed AI sessions route between pilot, rescue, Studio publish, checked reply refresh, and safe speech from one control receipt.
70. Add `best-friend-operate` so the model can execute one control-selected offline branch, refresh control, and stop before live publish or a second move.
71. Add `best-friend-runner` so the model can run bounded operator steps until checked speech, Studio publish, blockers, repeated branch risk, or a step limit stops it.
72. Make Studio review publishing best-friend-runner-aware so creators can inspect bounded supervisor status, stop reason, step count, checked message, and next action inside Studio.
19. Add evidence kits that make live proof collection concrete and repeatable.
20. Add review packs that make creator approval and AI review one artifact-backed surface.
21. Add place surveys that ground planning in the current Studio/project state.
22. Add scout packets that merge the creator request with survey evidence into one next build move.
23. Add session packets that bootstrap the scout decision into a complete offline work order.
24. Add live-gate packets that make the final pre-apply go/no-go decision explicit.
25. Add closeout packets that prevent false done claims and point to the remaining proof.
26. Add timeline packets that make every run resumable from one black-box artifact.
27. Add drive packets that safely orchestrate the offline run through review, approval, closeout, and timeline without mutating Studio.
28. Add pitch boards that let an AI offer ranked, safe, playable directions before driving one.
29. Add storyboards that translate recipes into player-facing loops, UI surfaces, demo scripts, and proof expectations.
30. Add proposal packets that combine recommendation, alternatives, storyboard, and claim boundaries into one creator review artifact.
31. Add selection packets that record the creator's chosen proposal candidate before a run is driven.
32. Add launch packets that consume a selection and drive the chosen run to the offline mutation boundary.
33. Auto-start the local bridge during readiness gates so AI agents can surface the real Studio/plugin blocker.
34. Add setup packets that convert readiness blockers into AI-safe repair actions and claim guardrails.
35. Add setup `--fix` so agents can perform the safe local plugin install step and preserve restart evidence.
36. Add companion packets that combine creator proposal, setup readiness, next actions, and claim guardrails into one AI handoff.
37. Add Studio review publishing so offline AI packets are visible inside Roblox Studio before mutation.
38. Make kickoff write project memory so every first run is resumable without an extra command.
39. Add static player-journey simulation so agents can catch disconnected gameplay before live apply.
40. Add feature graphs so agents can reason about connected systems instead of flat operation lists.
41. Add rehearsal runbooks that turn proof packets into a safe live-demo script for AI-led creator sessions.
42. Add balance intelligence so agents can catch economy dead ends before live apply.
43. Add impact maps so agents can see services, scripts, remotes, cloud surfaces, and rollback pressure before approval.
44. Add remote contract maps so agents can understand client/server APIs before patching or apply.
45. Add server authority audits so agents can review exploit-sensitive generated surfaces before approval.
45. Add player UX audits so agents can verify affordances, feedback, onboarding copy, and readability.
46. Add copy decks so agents can review generated text and localization readiness.
47. Add performance budget audits so agents can catch heavy generated slices before live apply.
48. Add accessibility audits so agents can catch inclusive-design risks before live apply.
49. Add policy/safety audits so agents can catch Roblox-sensitive generated surfaces before live apply.
50. Add intent traceability so agents can prove prompt coverage from recipes to artifacts.
51. Add offline packet refresh so agents can heal stale run artifacts in one command.
52. Add style guides so agents keep assets, UI, audio, copy, and thumbnails creatively coherent.
53. Add asset production briefs so agents can coordinate UI art, models, audio, VFX, thumbnails, and import commands from one run artifact.
54. Add world blueprints so agents can reason about zones, routes, anchors, screenshots, and playtest proof in space.
55. Add onboarding plans so agents can prove the first player session teaches the generated loop.
56. Add showcase plans so agents can turn generated runs into screenshot, thumbnail, trailer, talking-point, and proof capture guidance.
57. Add telemetry plans so agents can define anonymous events, funnels, retention hooks, and privacy guardrails before shipping blind.
58. Add monetization plans so agents can propose ethical offers, commerce surfaces, price tests, and trust guardrails without inventing live marketplace state.
59. Add social plans so agents can design optional friend loops, community hooks, and growth proof without spam, off-platform prompts, or unverified social claims.
60. Add liveops plans so agents can plan safe updates, events, experiments, and proof gates without pretending content is published or tested.
61. Add persistence plans so agents can define save schemas, migrations, and data-loss guardrails before claiming progress is safe.
62. Add evidence reviews so agents can turn live playtest observations into repair hypotheses and honest claim boundaries.
63. Add path reconciliation so agents can compare run plans with fresh Studio evidence before claiming applied work is still present.
64. Add publish prep so agents can draft Roblox-facing release materials and blockers without pretending the game is published.
65. Add creator feedback triage so agents can route review notes into patch lanes without pretending the feedback was implemented.
66. Add feedback patch work orders so agents can turn triaged notes into strict planner packets without guessing the next build step.
67. Add claim checks so agents can verify proposed user-facing claims before reporting.
68. Add evidence-gated response composition so agents can send only proof-backed creator updates.
69. Add creator decision ledgers so agents can preserve choices, constraints, and rejections without treating them as live approval.
70. Add decision alignment gates so agents can check plans against creator choices before continuing.
71. Make next-move selection alignment-aware so decision drift blocks the AI command queue.
72. Make cockpit and capsule handoffs decision-aware so creator intent is required context for resumed agents.
73. Add AI work journals so resumed agents inherit attempted commands, results, and evidence pointers without treating notes as proof.
74. Add opportunity maps so agents can compare high-leverage build, repair, proof, and continuity moves before choosing.
75. Add AI work orders so the selected opportunity becomes exact execution steps, validation commands, stop conditions, and claim guardrails.
76. Add AI work checks so selected work orders cannot be reported until expected artifacts and bundle evidence are present.
77. Add AI cycle packets so agents can move from opportunity to work order to evidence check to safe response without stale chat memory.
78. Add AI failure diagnosis so stuck cycles, failed commands, stale bundles, missing evidence, and claim blockers become exact recovery commands.
79. Add safe offline action running so agents can execute one whitelisted next action and refresh evidence without crossing live boundaries.
80. Add a guarded offline loop so agents can repeat cycle/act until ready-to-report or blocked without babysitting every step.
81. Add a creator message router so raw chat becomes safe feedback, decision, rollback, status, build, or approval-gate commands before an AI acts.
82. Add a safe creator message handler so the first supported offline route can execute and preserve live/apply wording as a gate.
83. Add durable conversation state so resumed agents know handled turns, open loops, safe claims, and next commands from artifacts.
84. Add a one-message AI chat operator so agents can safely handle creator chat, create feedback work orders, refresh conversation state, and prepare checked replies.
85. Add creator promise fulfillment audits so agents map requested features to proof before saying work is done.
86. Add creator promise satisfiers so missing deterministic promise gaps become offline patch runs with comparison and sequence artifacts.
87. Add multi-step promise loops so agents keep creating offline patch runs until deterministic creator-promise recipes are covered.
88. Make the AI command queue promise-aware so missing deterministic creator promises route through `next`, `opportunities`, `cycle`, and `act`.

The product becomes an AI's best friend when an agent can inspect a place, propose a feature, apply it safely, verify it, explain the result, and undo it without leaving the creator guessing.
