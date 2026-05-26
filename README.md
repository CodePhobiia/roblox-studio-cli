# rs - Roblox Studio CLI

A single-binary CLI plus Luau Studio plugin for programmatic Roblox Studio control.

Private alpha first run: follow the proof-bound starter shop path in [Getting Started Alpha](docs/getting-started-alpha.md).

## What It Does

| Command | Purpose |
|---|---|
| `rs list` | Show connected Studios |
| `rs doctor` | Diagnose bridge, plugin install, connected Studios, and protocol mismatches |
| `rs install-plugin` | Build, copy, hash-check, and report Studio windows that need restart |
| `rs exec --studio X --allow-dangerous-exec --lua "code"` | Run trusted Luau in a Studio and return JSON |
| `rs read --studio X --path Workspace --depth 2` | Read a rich JSON instance tree |
| `rs export --studio X --path ServerStorage.Foo --out ./export` | Save a subtree as individual local files |
| `rs import-asset --studio X --file ./mesh.obj --to Workspace` | Convert a local mesh asset into welded MeshParts |
| `rs import-image --studio X --file ./icon.png --kind button --to StarterGui` | Import a local PNG as GUI UI |
| `rs import-ui-pack --studio X --folder ./ui --to StarterGui` | Import a folder or manifest of PNG UI elements |
| `rs import-audio --studio X --file ./click.wav --asset-id rbxassetid://123 --to SoundService` | Create Sound instances from Roblox audio asset IDs |
| `rs auth profile add mygroup --creator-id 123 --api-key ...` | Save a local Open Cloud upload profile |
| `rs upload image ./icon.png --profile mygroup --wait` | Upload images, audio, or model files through Roblox Open Cloud and wait for final asset ID |
| `rs import-uploaded image rbxassetid://123 --to StarterGui` | Import an already uploaded image/audio asset ID into Studio |
| `rs validate --studio X --path Workspace.Tool` | Report broken refs, Tool wiring, asset, and path issues |
| `rs validate --studio X --path Workspace.Tool --fix` | Run validation, apply safe repairs, then validate again |
| `rs repair-tool --studio X --path Workspace.Tool` | Weld loose Tool parts to the Handle and fix equip-ready physics |
| `rs smoke regression --studio X --out smoke.json --upload-mock` | Run the broader live regression suite and save a JSON report |
| `rs snapshot --studio X --path Workspace` | Summarize a subtree inventory |
| `rs create --studio X --class Folder --to ReplicatedStorage --name Shared` | Create an Instance with optional typed properties |
| `rs diff --export ./old --against-export ./new --fix-plan` | Compare sources and emit a safe mutation plan |
| `rs apply-plan --file plan.json --root Workspace.Foo --dry-run` | Preview or apply approved safe diff plan operations |
| `rs sync pull --studio X --path Workspace.Foo --out ./pulled` | Pull Studio changes back to disk with metadata and transfer blob |
| `rs sync-folder --studio X --folder ./src --to ServerScriptService` | Push local scripts and assets into Studio |
| `rs batch --file rs.batch.json` | Run multiple operations from one manifest |
| `rs package --studio X --path ServerStorage.Foo --out ./foo.rspkg` | Create portable packages with export data and transfer blobs |
| `rs package verify --file ./foo.rspkg` | Verify package manifest, checksums, transfer blob, asset refs, and optional conflicts |
| `rs package update --file ./foo.rspkg --to Workspace --owned-only` | Reapply a package using stable ownership metadata |
| `rs package pack ./foo.rspkg --out foo.rspkg.zip` | Pack/unpack portable package archives |
| `rs package import --file ./foo.rspkg --if-exists fail|replace|merge|rename --dry-run` | Safely plan or import packages into Studio |
| `rs package import --file ./foo.rspkg --rehost-images --profile targetgroup` | Reupload referenced images to the target creator and rewrite UI refs before import |
| `rs transaction snapshot --path Workspace.Foo --out foo.snapshot.json` | Capture or restore rollback transfer snapshots |
| `rs history` / `rs undo <id> --yes` | Inspect Studio-side command audit history and undo when a snapshot exists |
| `rs deps --path Workspace.Tool --format json` | Report asset, script, remote, and ownership dependencies |
| `rs publish-check --path Workspace.Tool` | Run shipping preflight checks for refs, assets, ownership, protocol, and package drift |
| `rs autopilot recipes --format json` | List deterministic Roblox feature recipes and their generated files |
| `rs autopilot capabilities --format json` | Write an AI-readable atlas of verified recipes, workflows, commands, artifacts, and safety boundaries |
| `rs autopilot tune "make a candy tycoon" --smoke regression` | Generate an explicit tuned compose manifest with recipe names, economy knobs, and next commands |
| `rs autopilot compose --from-manifest .rs\autopilot\manifests\game.autopilot-compose.json` | Build a reviewable offline run from recipe presets or tuned manifests |
| `rs autopilot control --root .rs\autopilot\runs` | Write one AI mission-control packet with trusted evidence, next command, and do-not-claim guardrails |
| `rs autopilot brief --run-dir .rs\autopilot\runs\tycoon` | Write a creator-safe status brief with allowed claims, forbidden claims, evidence, and next actions |
| `rs autopilot inbox "shop button is confusing" --run-dir .rs\autopilot\runs\tycoon` | Route a creator message into safe feedback, decision, rollback, status, build, or approval-gate commands |
| `rs autopilot handle "shop button is confusing" --run-dir .rs\autopilot\runs\tycoon` | Route and execute one safe offline creator-message action, or stop at approval/live gates |
| `rs autopilot conversation --run-dir .rs\autopilot\runs\tycoon` | Summarize creator/AI turns, handled artifacts, open loops, safe claims, and next commands |
| `rs autopilot chat "shop button is confusing" --run-dir .rs\autopilot\runs\tycoon` | Handle creator chat, prepare feedback work orders, refresh conversation state, and draft checked replies without live mutation |
| `rs autopilot intake "make a tycoon"` | Interpret a creator request into intent, assumptions, clarifying questions, acceptance criteria, and first commands |
| `rs autopilot start "make a tycoon"` | Run the offline AI startup flow from intake through kickoff, mission control, user brief, and review pack |
| `rs autopilot pitch "make a tycoon"` | Offer ranked deterministic build directions and exact drive commands before choosing one |
| `rs autopilot storyboard "make a tycoon"` | Write the player promise, core loop, UI surfaces, demo script, and proof expectations |
| `rs autopilot proposal "make a tycoon"` | Combine pitch, storyboard, safe claims, and next commands into one creator review packet |
| `rs autopilot companion "make a tycoon"` | Write one proposal plus setup-readiness companion packet for AI handoff |
| `rs autopilot select .rs\autopilot\proposal.json` | Record the chosen proposal candidate and exact drive command before build orchestration |
| `rs autopilot launch .rs\autopilot\selection.json` | Drive the chosen proposal through safe offline gates without mutating Studio |
| `rs autopilot setup --fix --format json` | Write an AI-readable bridge, Studio, and plugin readiness repair packet, optionally installing the plugin bundle |
| `rs autopilot drive "make a tycoon"` | Bootstrap or resume an AI run through safe offline gates and stop before Studio mutation |
| `rs autopilot survey --context .rs\autopilot\context\context.json` | Turn live context into an AI-readable place map with risks, safe zones, recipes, and next commands |
| `rs autopilot reconcile .rs\autopilot\runs\tycoon --context .rs\autopilot\context\context.json` | Compare planned/applied Studio paths with live evidence and flag aligned, missing, or drifted work |
| `rs autopilot scout "add a shop" --survey .rs\autopilot\survey.json` | Combine a creator request and place survey into the next safe AI build move |
| `rs autopilot session "make a tycoon" --survey .rs\autopilot\survey.json` | Turn scout evidence into a full offline AI work session with review, evidence, approval, and capsule artifacts |
| `rs autopilot live-gate --run-dir .rs\autopilot\runs\tycoon` | Produce the final go/no-go packet before any approved live apply |
| `rs autopilot rehearsal .rs\autopilot\runs\tycoon` | Write a non-mutating approval, readiness, apply, proof, and closeout runbook for an AI-led live demo |
| `rs autopilot closeout .rs\autopilot\runs\tycoon` | Produce an honest done/not-done verdict from proof, acceptance, judgment, rollback, and playtest artifacts |
| `rs autopilot timeline .rs\autopilot\runs\tycoon` | Write a black-box run timeline with artifact status, stale-proof warnings, and the safest resume command |
| `rs autopilot cockpit --run-dir .rs\autopilot\runs\tycoon` | Write one AI dashboard with mission control, proof, decisions, alignment, journal, roadmap, memory, evidence, and command queue |
| `rs autopilot capsule --run-dir .rs\autopilot\runs\tycoon` | Write a copy/paste-safe AI continuation packet with required decision/alignment/journal context, resume prompt, privacy status, and guardrails |
| `rs autopilot orient --run-dir .rs\autopilot\runs\tycoon` | Write one cold-start AI orientation packet with capability atlas, playbook, read order, safe claims, and the exact next command |
| `rs autopilot model-pack .rs\autopilot\runs\tycoon` | Write a redacted model-ready context pack with resume prompt, embedded snippets, source links, and next safe actions |
| `rs autopilot task-pack .rs\autopilot\runs\tycoon` | Write a copy/paste-ready AI task prompt with allowed commands, validation, stop rules, model context, and work-order proof |
| `rs autopilot best-friend .rs\autopilot\runs\tycoon --root .rs\autopilot\runs` | Launch a fresh AI companion with remembered context, checked opening reply, launch self-check, and protected first action |
| `rs autopilot best-friend-check .rs\autopilot\runs\tycoon --root .rs\autopilot\runs` | Audit launch readiness before a fresh AI speaks or acts, including memory, read order, preflight, protected action, and Studio review visibility |
| `rs autopilot best-friend-rescue .rs\autopilot\runs\tycoon --result "failed"` | Recover a blocked companion session with launch-control, optional diagnosis, a selected repair command, and checked recovery wording |
| `rs autopilot best-friend-mentor .rs\autopilot\runs\tycoon --root .rs\autopilot\runs` | Coach a fresh AI on what to read, why the next move matters, which mistakes to avoid, and when to rescue before acting |
| `rs autopilot best-friend-pilot .rs\autopilot\runs\tycoon --root .rs\autopilot\runs` | Coach the companion, run one protected offline action when safe, and prepare the checked creator-facing reply |
| `rs autopilot best-friend-control .rs\autopilot\runs\tycoon --root .rs\autopilot\runs` | Decide whether the AI should pilot, rescue, publish the co-pilot receipt to Studio, or send the checked reply |
| `rs autopilot best-friend-operate .rs\autopilot\runs\tycoon --root .rs\autopilot\runs` | Execute one control-selected offline branch, refresh control, and stop before Studio publish or a second move |
| `rs autopilot best-friend-runner .rs\autopilot\runs\tycoon --root .rs\autopilot\runs --max-steps 3` | Run bounded operator steps until a checked reply, Studio publish, blocker, repeat, or step limit stops the AI |
| `rs autopilot first-turn .rs\autopilot\runs\tycoon --root .rs\autopilot\runs` | Execute exactly one protected best-friend first action through `act`, then refresh the launch packet with a receipt |
| `rs autopilot best-friend-loop .rs\autopilot\runs\tycoon --max-steps 3` | Run bounded protected best-friend turns and stop before repeats, blockers, or live gates |
| `rs autopilot best-friend-reply .rs\autopilot\runs\tycoon` | Draft and self-check the creator-facing update after best-friend work |
| `rs autopilot best-friend-turn .rs\autopilot\runs\tycoon --message "make the shop button brighter"` | Handle one creator message, run protected best-friend work, and prepare the checked reply as one receipt |
| `rs autopilot best-friend-session "make a tycoon with shop and saves"` | Bootstrap or resume a companion session, run the protected turn, and return the checked message |
| `rs autopilot wow-session "make a tycoon with shop and saves" --assume` | Bootstrap or resume a companion session, build the offline wow candidate, and prepare the proof-bound creator demo |
| `rs autopilot best-friend-arc "make a tycoon with shop and saves" --message "Looks good but make the shop button brighter"` | Run the whole companion arc from first wow demo to checked post-demo reply and remembered feedback |
| `rs autopilot squad-pack .rs\autopilot\runs\tycoon` | Split the opportunity map into parallel AI assignments with ownership, allowed commands, validation, and stop rules |
| `rs autopilot squad-review .rs\autopilot\runs\tycoon` | Reconcile squad assignments, expected artifacts, journal evidence, ownership conflicts, and integration next steps |
| `rs autopilot wow-plan .rs\autopilot\runs\tycoon` | Rank safe, non-mutating wow-factor candidates with player moments, proof needs, and next commands |
| `rs autopilot moment-pack .rs\autopilot\runs\tycoon` | Turn the selected wow moment into agent-ready implementation lanes, proof checks, and stop conditions |
| `rs autopilot moment-sprint .rs\autopilot\runs\tycoon` | Build the selected wow moment as a separate reviewed offline candidate run without touching Studio |
| `rs autopilot moment-decision .rs\autopilot\runs\tycoon` | Compare the wow candidate against the source run and recommend the proof-bound continuation |
| `rs autopilot creator-demo .rs\autopilot\runs\tycoon` | Prepare a proof-bound creator demo packet for the recommended wow run |
| `rs autopilot demo-response .rs\autopilot\runs\tycoon --message "Looks good but make the shop button brighter"` | Route post-demo creator approval, feedback, redirection, or questions into the next safe artifact path |
| `rs autopilot demo-loop .rs\autopilot\runs\tycoon --message "Looks good but make the shop button brighter"` | Package the routed demo response into the next AI handoff prompt, command queue, and stop rules |
| `rs autopilot demo-session .rs\autopilot\runs\tycoon --message "Looks good but make the shop button brighter"` | Route a post-demo reaction, check the reply, learn from it, and refresh remembered AI context |
| `rs autopilot demo-check .rs\autopilot\runs\tycoon` | Audit post-demo follow-through before claiming feedback, approval, redirection, or replies are handled |
| `rs autopilot demo-reply .rs\autopilot\runs\tycoon` | Compose an evidence-checked creator reply from the demo-check state |
| `rs autopilot demo-learn .rs\autopilot\runs\tycoon` | Distill post-demo creator reactions into reusable taste, constraint, and follow-up signals |
| `rs autopilot remember .rs\autopilot\runs\tycoon --root .rs\autopilot\runs --best-friend` | Consolidate demo learning into refreshed memory, preferences, game bible, and AI playbook context, then optionally write the checked `best-friend` launch packet |
| `rs autopilot review-pack .rs\autopilot\runs\tycoon` | Refresh approval, proof, privacy, evidence, and capsule artifacts into one creator review packet |
| `rs autopilot publish-review .rs\autopilot\runs\tycoon --arc .rs\autopilot\runs\tycoon\best-friend-arc.json --best-friend .rs\autopilot\runs\tycoon\best-friend.json --best-friend-pilot .rs\autopilot\runs\tycoon\best-friend-pilot.json --best-friend-runner .rs\autopilot\runs\tycoon\best-friend-runner.json` | Publish an offline review packet, creator arc, AI launch context, checked co-pilot result, and bounded runner state into the Studio Autopilot panel without mutating the place |
| `rs autopilot publish-prep .rs\autopilot\runs\tycoon` | Draft Roblox-facing store copy, launch checks, asset needs, and release blockers without publishing |
| `rs autopilot feedback .rs\autopilot\runs\tycoon --note "shop button is confusing"` | Turn creator review notes into categorized patch lanes, clarification questions, claim boundaries, and next commands |
| `rs autopilot feedback-patch .rs\autopilot\runs\tycoon` | Convert feedback triage into a strict AI patch work order and feedback-specific planner pack |
| `rs autopilot claim-check .rs\autopilot\runs\tycoon --claim "The patch work order is ready"` | Check proposed creator-facing claims against run evidence and safe-response guardrails |
| `rs autopilot respond .rs\autopilot\runs\tycoon --claim "The run has a valid plan"` | Compose a creator-facing response only after the claims pass evidence checks |
| `rs autopilot decision .rs\autopilot\runs\tycoon --decision "Use the cozy shop direction" --constraint "Keep combat nonviolent"` | Preserve creator decisions, constraints, rejections, and notes without treating them as live approval |
| `rs autopilot preferences --root .rs\autopilot\runs` | Learn cross-run creator preferences, constraints, rejections, feedback themes, and planning guidance |
| `rs autopilot game-bible --root .rs\autopilot\runs` | Fuse memory, preferences, style, and world artifacts into one cross-run project canon |
| `rs autopilot playbook --root .rs\autopilot\runs` | Build the cross-run AI operating playbook from canon, preferences, memory, and retrospectives |
| `rs autopilot director "add pets" --root .rs\autopilot\runs` | Rank canon-aware creative build bets with exact safe offline commands and proof needs |
| `rs autopilot pursue "add pets" --root .rs\autopilot\runs` | Execute the first supported safe director bet through internal offline handlers |
| `rs autopilot agenda "add pets" --root .rs\autopilot\runs` | Distill the AI cockpit into a prioritized, claim-safe work agenda |
| `rs autopilot sprint "add pets" --run-dir .rs\autopilot\runs\tycoon` | Run a bounded agenda sprint through safe internal offline handlers and stop at live gates |
| `rs autopilot retrospect .rs\autopilot\runs\tycoon` | Convert recent run evidence into AI lessons, safe claims, and next commands |
| `rs autopilot align .rs\autopilot\runs\tycoon` | Check `plan.json` against recorded creator decisions before the next AI continues |
| `rs autopilot journal .rs\autopilot\runs\tycoon --entry "Aligned the plan and found no drift"` | Preserve AI work notes, attempted commands, results, and evidence pointers for the next session |
| `rs autopilot proof .rs\autopilot\runs\tycoon` | Write a proof ledger mapping allowed claims to concrete artifacts and missing evidence |
| `rs autopilot acceptance .rs\autopilot\runs\tycoon` | Score whether a run satisfies the creator request, separating offline readiness from live proof |
| `rs autopilot fulfillment .rs\autopilot\runs\tycoon` | Map creator promises to evidence, missing gaps, live proof, and safe next actions |
| `rs autopilot completion-audit .rs\autopilot\runs\tycoon` | Build the prompt-to-artifact done/not-done checklist before reporting completion |
| `rs autopilot deliver .rs\autopilot\runs\tycoon` | Write the creator-facing delivery message from completion-audit evidence |
| `rs autopilot satisfy .rs\autopilot\runs\shop "make a shop with coins"` | Convert missing creator-promise recipe gaps into an offline patch run with comparison and sequence artifacts |
| `rs autopilot promise-loop .rs\autopilot\runs\shop "make a shop with coins and quests"` | Keep creating offline patch runs until deterministic creator-promise recipes are covered in one sequence |
| `rs autopilot rollback .rs\autopilot\runs\tycoon` | Write a rollback readiness packet with verified undo artifact state and restore commands |
| `rs autopilot approval .rs\autopilot\runs\tycoon` | Write the creator approval prompt, live-readiness command, and exact approved apply command |
| `rs autopilot privacy .rs\autopilot\runs\tycoon` | Scan run artifacts for unredacted secret-like values before AI handoff |
| `rs autopilot next --root .rs\autopilot\runs` | Choose the safest next agent action from decisions, alignment, creator promises, memory, gameplay, bundle, and certification evidence |
| `rs autopilot opportunities --run-dir .rs\autopilot\runs\tycoon` | Rank evidence-backed build, repair, fulfillment, proof, and continuity opportunities with commands and expected artifacts |
| `rs autopilot work-order --run-dir .rs\autopilot\runs\tycoon` | Turn the selected opportunity into exact execution steps, validation commands, stop conditions, and claim guardrails |
| `rs autopilot work-check .rs\autopilot\runs\tycoon` | Check the selected work order's expected artifacts and bundle state before reporting progress |
| `rs autopilot cycle .rs\autopilot\runs\tycoon` | Run one offline AI loop through opportunities, work order, work check, claim-check, and response routing |
| `rs autopilot diagnose .rs\autopilot\runs\tycoon --error "selected command has not been run yet"` | Diagnose stuck AI cycles, failed commands, missing evidence, stale bundles, and claim blockers |
| `rs autopilot command-guard --run-dir .rs\autopilot\runs\tycoon` | Validate an AI-proposed command sequence before execution and flag unsupported, live, or mutating steps |
| `rs autopilot self-check .rs\autopilot\runs\tycoon --claim "The offline run is ready for review" --command "rs autopilot proof .rs\autopilot\runs\tycoon --format json"` | Check proposed AI claims and commands together before speaking to the creator or acting |
| `rs autopilot runbook --run-dir .rs\autopilot\runs\tycoon` | Split a guarded command queue into a safe offline prefix, gated suffix, stop rules, and expected evidence |
| `rs autopilot flight-recorder .rs\autopilot\runs\tycoon` | Summarize command, gate, evidence, blocker, and claim history so an AI can resume from artifacts |
| `rs autopilot navigator --run-dir .rs\autopilot\runs\tycoon` | Write one concise AI operating card from orientation, runbook, recorder, stop rules, and safe claims |
| `rs autopilot advance .rs\autopilot\runs\tycoon` | Execute exactly one navigator-selected safe offline `act` action and refresh navigation evidence |
| `rs autopilot act .rs\autopilot\runs\tycoon` | Execute the next whitelisted offline Autopilot action, including promise repair commands, then refresh cycle/diagnosis and refuse live mutation |
| `rs autopilot loop .rs\autopilot\runs\tycoon --max-steps 3` | Keep cycling and acting through safe offline steps until ready to report or blocked |
| `rs autopilot roadmap "add quests" --root .rs\autopilot\runs` | Write a milestone backlog with exact commands and expected artifacts for multi-step AI work |
| `rs autopilot judge .rs\autopilot\runs\tycoon` | Produce an honest readiness verdict without claiming production-ready before live proof exists |
| `rs autopilot kickoff "make a tycoon"` | Create an offline AI-ready tuned manifest, plan, preview, audit, simulation, feature graph, balance, policy audit, style guide, world blueprint, onboarding, showcase, telemetry, monetization, social, liveops, persistence, asset brief, intent trace, handoff, certification, critique, playtest, and planner packet |
| `rs autopilot critique --run-dir .rs\autopilot\runs\tycoon` | Score a planned gameplay slice and recommend the next design-safe recipe patch |
| `rs autopilot playtest --run-dir .rs\autopilot\runs\tycoon` | Write recipe-aware live playtest steps and expected evidence for the generated slice |
| `rs autopilot simulate --run-dir .rs\autopilot\runs\tycoon` | Dry-run the static player journey and flag missing gameplay beats before live apply |
| `rs autopilot graph --run-dir .rs\autopilot\runs\tycoon` | Connect recipes, scripts, remotes, UI, and verification gates into an AI-readable feature graph |
| `rs autopilot balance --run-dir .rs\autopilot\runs\tycoon` | Analyze currencies, rewards, prices, starter balance, and first-purchase pacing before live apply |
| `rs autopilot impact .rs\autopilot\runs\tycoon` | Map services, scripts, remotes, cloud surfaces, approval pressure, and rollback blast radius |
| `rs autopilot contracts .rs\autopilot\runs\tycoon` | Map RemoteEvent and RemoteFunction client/server contracts across generated scripts |
| `rs autopilot authority .rs\autopilot\runs\tycoon` | Audit server authority, client mutation risk, and exploit-sensitive generated surfaces |
| `rs autopilot ux .rs\autopilot\runs\tycoon` | Audit generated UI, player actions, feedback loops, onboarding copy, and readability |
| `rs autopilot copy-deck .rs\autopilot\runs\tycoon` | Extract generated player-facing text for review, tuning, and localization readiness |
| `rs autopilot performance .rs\autopilot\runs\tycoon` | Audit generated instance, script, loop, remote, and source-size budgets before live apply |
| `rs autopilot accessibility .rs\autopilot\runs\tycoon` | Audit scalable text, touch targets, contrast, input affordances, and motion-sensitive patterns |
| `rs autopilot policy .rs\autopilot\runs\tycoon` | Audit generated purchases, persistence, teleports, HTTP, chat, personal data, and off-platform link risks |
| `rs autopilot style-guide .rs\autopilot\runs\tycoon` | Write a theme, palette, tone, UI, copy, audio, and asset style bible for coherent future assets |
| `rs autopilot world-blueprint .rs\autopilot\runs\tycoon` | Write zones, player routes, interaction anchors, camera shots, and spatial build rules |
| `rs autopilot onboarding .rs\autopilot\runs\tycoon` | Write first-session steps, teaching prompts, and proof checks for the generated loop |
| `rs autopilot showcase .rs\autopilot\runs\tycoon` | Write screenshot, thumbnail, trailer, talking-point, and proof guidance for creator demos |
| `rs autopilot telemetry .rs\autopilot\runs\tycoon` | Write analytics events, funnels, retention hooks, and privacy guardrails for the generated loop |
| `rs autopilot monetization .rs\autopilot\runs\tycoon` | Write ethical offer candidates, commerce surfaces, price-test ideas, and trust guardrails |
| `rs autopilot social .rs\autopilot\runs\tycoon` | Write social loops, friend moments, community hooks, and proof guardrails |
| `rs autopilot liveops .rs\autopilot\runs\tycoon` | Write update cadence, event hooks, experiments, and operational proof gates |
| `rs autopilot persistence .rs\autopilot\runs\tycoon` | Write DataStore schema, save/load flows, migrations, and data-loss guardrails |
| `rs autopilot asset-brief .rs\autopilot\runs\tycoon` | Write UI art, model, audio, VFX, and thumbnail production requests with import/upload commands |
| `rs autopilot trace .rs\autopilot\runs\tycoon` | Map the creator prompt to expected recipes, generated files, and review artifacts |
| `rs autopilot refresh .rs\autopilot\runs\tycoon` | Rebuild stale or missing offline review artifacts for an existing run |
| `rs autopilot evidence .rs\autopilot\runs\tycoon` | Create an evidence folder layout and record-playtest commands for live proof collection |
| `rs autopilot record-playtest .rs\autopilot\runs\tycoon --result passed --evidence "cash upgraded in Play Solo"` | Record live playtest proof for later `judge` decisions |
| `rs autopilot evidence-review .rs\autopilot\runs\tycoon` | Diagnose recorded live evidence into scenario observations, claim boundaries, and repair hypotheses |
| `rs autopilot health .rs\autopilot\runs\tycoon` | Decide whether an applied run is healthy using apply, validation, regression smoke, rollback, proof, and playtest evidence |
| `rs autopilot repair-plan .rs\autopilot\runs\tycoon` | Convert failed playtest evidence into an AI-ready repair packet |
| `rs autopilot improve --run-dir .rs\autopilot\runs\shop` | Turn critique gaps into a fresh offline patch run with review, certification, critique, and playtest artifacts |
| `rs autopilot compare --base-run .rs\autopilot\runs\shop --candidate-run .rs\autopilot\runs\shop-tycoon` | Compare runs and recommend the safer continuation from score, recipe, path, bundle, and blocker deltas |
| `rs autopilot iterate --run-dir .rs\autopilot\runs\shop` | Run the offline critique-improve-compare loop until a playable candidate or custom-plan blocker is found |
| `rs autopilot sequence --run-dir .rs\autopilot\runs\shop --run-dir .rs\autopilot\runs\shop-tycoon` | Write an ordered apply packet for baseline and patch runs |
| `rs transfer --from "A:Path" --to "B:ParentPath" --rehost-images --profile targetgroup` | Copy or plan an instance-tree transfer, optionally rehosting image refs |
| `rs bridge serve/status/stop` | Manage the local bridge daemon |

The bridge listens on `127.0.0.1:7878` by default and can be changed with `--port` or `RS_BRIDGE_PORT`. CLI-facing bridge routes require a local token sent by the `rs` binary. Set `RS_BRIDGE_TOKEN` only for trusted local tooling that must call the bridge directly.

Transfers validate refs, welds, and Tools after deserialize. If a weld or attachment constraint points outside the selected source root, transfer a common parent containing both endpoints or pass `--allow-external-refs` to accept the missing link intentionally.

## Build

```powershell
cargo build --release
```

The binary will be at `target\release\rs.exe`.

## Install The Studio Plugin

```powershell
cd plugin
rojo build default.project.json --output rs-bridge-plugin.rbxmx
```

Drop `rs-bridge-plugin.rbxmx` into your local Roblox Studio plugins folder:

- Windows: `%LOCALAPPDATA%\Roblox\Plugins\`
- macOS: `~/Documents/Roblox/Plugins/`

Restart Studio. The plugin registers with the local bridge and starts polling for commands.

## Quick Start

```powershell
rs list
rs exec --studio "Snipe a Slime!" --allow-dangerous-exec --lua "return #game.ReplicatedStorage:GetChildren()"
rs read --studio "Snipe a Slime!" --path "Workspace" --depth 1
rs export --studio "Snipe a Slime!" --path "ServerStorage.SniperSkins" --out ".\exports\snipers"
rs import-asset --studio "Snipe a Slime!" --file ".\assets\crate.obj" --to "Workspace" --name "Crate"
rs import-image --studio "Snipe a Slime!" --file ".\assets\shop.png" --to "StarterGui" --kind button --size 96x96
rs import-ui-pack --studio "Snipe a Slime!" --folder ".\ui\shop" --to "StarterGui" --name "ShopGui"
rs install-plugin
rs doctor
rs validate --studio "Snipe a Slime!" --path "Workspace.Crate" --fix
rs repair-tool --studio "Snipe a Slime!" --path "Workspace.Rifle"
rs smoke all --studio "Snipe a Slime!"
rs smoke regression --studio "Snipe a Slime!" --out ".\smoke-regression.json" --upload-mock
rs snapshot --studio "Snipe a Slime!" --path "Workspace" --format json
rs auth profile add mygroup --creator-id 123456 --creator-type group --api-key $env:ROBLOX_API_KEY
rs upload image ".\assets\shop.png" --profile mygroup --wait --import-to StarterGui --studio "Snipe a Slime!"
rs import-uploaded audio rbxassetid://1234567890 --studio "Snipe a Slime!" --to SoundService --name Click
rs diff --export ".\exports\old" --against-export ".\exports\new" --fix-plan --format json > ".\plan.json"
rs apply-plan --studio "Snipe a Slime!" --root Workspace.Crate --file ".\plan.json" --dry-run --only added,modified --exclude Scripts
rs sync pull --studio "Snipe a Slime!" --path "Workspace.Crate" --out ".\pulled\crate" --overwrite
rs package --studio "Snipe a Slime!" --path "ServerStorage.SniperSkins" --out ".\packages\sniper-skins.rspkg"
rs package verify --file ".\packages\sniper-skins.rspkg"
rs package pack ".\packages\sniper-skins.rspkg" --out ".\packages\sniper-skins.rspkg.zip"
rs package import --studio "Snipe a Slime!" --file ".\packages\sniper-skins.rspkg" --to ServerStorage --if-exists rename --dry-run
rs package import --studio "Snipe a Slime!" --file ".\packages\sniper-skins.rspkg" --to ServerStorage --if-exists rename --rehost-images --profile mygroup
rs package update --studio "Snipe a Slime!" --file ".\packages\sniper-skins.rspkg" --to ServerStorage --owned-only --dry-run
rs deps --studio "Snipe a Slime!" --path "Workspace.Rifle"
rs publish-check --studio "Snipe a Slime!" --path "Workspace.Rifle" --package ".\packages\sniper-skins.rspkg"
rs transfer --from "Snipe for Brainrots!:StarterGui.ShopGui" --to "Snipe a Slime!:StarterGui" --rehost-images --profile mygroup
```

The bridge auto-spawns on first CLI command and stays running until `rs bridge stop`.

`export` writes one `instance.json` metadata file per Studio instance. Scripts are emitted as
`.server.lua`, `.client.lua`, or `.module.lua`. Roblox-hosted meshes, textures, images,
audio, animations, and VFX textures are emitted as individual `.asset.json` reference files
containing the source property and asset URI.

`doctor` checks bridge liveness, connected Studios, the installed plugin bundle timestamp, and
plugin/CLI protocol compatibility. `rs doctor --fix` starts the bridge when needed, copies the
repo-built plugin bundle into the local Roblox Plugins folder when it is stale, then re-runs the
diagnostics and prints exact restart/install guidance for old Studio sessions.

`install-plugin` is the more direct plugin workflow. It runs `rojo build`, copies
`plugin/rs-bridge-plugin.rbxmx` into the local Roblox Plugins folder, verifies the installed hash
and timestamp, and lists the exact connected Studio windows that must be restarted. `--watch` keeps
that build/copy loop running for plugin development.

`import-asset` natively supports local `.obj`, `.stl`, `.gltf`, and `.glb` static geometry.
For other Blender-readable formats such as `.fbx`, `.dae`, `.ply`, `.abc`, `.usd`, and `.blend`,
the CLI attempts a headless Blender conversion to GLB first. It then converts faces to triangles,
sends the mesh payload to the Studio plugin, and the plugin builds MeshParts through `EditableMesh`.
Multi-object files become multiple MeshParts welded together by default. OBJ/glTF material names,
base colors, texture URI metadata, object hierarchy paths, and source pivots are preserved as Studio
properties, `SurfaceAppearance` content where possible, or `rs*` attributes where Roblox does not
allow direct assignment. `--texture-root` can point at a folder with texture files or an
`rs-textures.json` mapping from source texture names to Roblox asset URIs. This is a Studio-local
import path; it does not upload permanent cloud mesh assets, rigs, skin weights, or animations.

`import-image` supports local `.png` files. The CLI decodes the PNG to RGBA pixels, downscales
images larger than Roblox's editable-image size limit, and sends the pixels to Studio. The plugin
creates an `EditableImage`, assigns it to `ImageContent`, and inserts either an `ImageLabel`,
`ImageButton`, or icon-sized `ImageLabel`. When the target parent is `StarterGui` or `PlayerGui`,
the plugin creates a `ScreenGui` container automatically.

`upload` publishes images, audio, and supported model containers with Roblox Open Cloud. `--wait`
polls the returned operation until the final `rbxassetid://...` is available. `--import-to` composes
upload, wait, and Studio import for image/audio assets. Profiles saved with `auth profile add` can
provide creator and API-key details. Raw local OBJ/STL geometry should use the Studio-local importer
unless you first convert it to an Open Cloud-supported model upload format such as FBX, glTF, or GLB.

`validate`, `repair-tool`, and `snapshot` provide the reliability layer around imports and
transfers. Validation reports nil weld/Motor6D/constraint references, Tool Handle and connectivity
issues, anchored Tool parts, empty common asset properties, and duplicate sibling names. Repair
preserves valid joints, optionally removes broken ones, creates missing `WeldConstraint`s to the
Handle, and can fix Tool physics settings. `validate --fix` runs validation, applies safe Tool
repairs when diagnostics advertise that fix, then validates again and reports before/after.

`diff`, `apply-plan`, `sync`, `sync-folder`, `batch`, and `package` are higher-level workflow commands.
UI packs assemble multiple PNGs into one ScreenGui. `smoke validate`, `smoke import-ui-pack`,
`smoke repair-tool`, `smoke all`, and `smoke regression` create live Studio fixtures, verify
behavior, and clean up. `diff --fix-plan` emits a conservative JSON mutation plan, and `apply-plan`
only applies approved safe operations with dry-run, kind filters, script exclusion, and ownership
checks. `sync-folder` pushes local scripts and can route PNG/mesh files through the existing
importers; `sync pull` writes Studio edits back to disk as script source, metadata JSON, asset refs,
and `transfer_blob.json`. Batch runs a JSON list of operations. Package writes a folder
containing an export tree, `transfer_blob.json`, validation data, a manifest, and checksums; package
import deserializes that transfer blob back into Studio. `package verify` checks manifests,
checksums, transfer blobs, asset-reference files, and optional Studio conflict dry-runs. `package
pack` and `package unpack` convert package folders to/from zip archives. Package import supports
`--dry-run`, `--rollback-on-error`, and `--if-exists fail|replace|merge|rename` so conflicts are
explicit before destructive changes. `--rehost-images` downloads referenced `Image`, `Texture`,
`TextureID`, and scrolling-frame image assets through Open Cloud, uploads target-owned copies, and
rewrites the in-memory transfer blob before import; use `--source-api-key` when source asset reads
need a different key than target uploads. `package update` reapplies packages by `rsSourceId` and
`rsPackageId` with owned-only, preserve-local, replace-owned, and conflict-report modes.

Studio imports, sync upserts, and package-created instances are stamped with `rsSourceId`,
`rsPackageId` when applicable, `rsImportedAt`, and `rsManagedBy` attributes to support safer future
merge/update workflows. Ownership-aware commands refuse to overwrite manual instances unless
`--force` is passed. Mutating handlers can record rollback snapshots in Studio-side command history;
`rs history`, `rs history show <id>`, and `rs undo <id> --yes` expose that audit log. `rs deps` and
`rs publish-check` provide the dependency graph and shipping preflight layer for meshes, textures,
images, audio, animations, scripts, remotes, missing asset IDs, unowned instances, broken welds, and
package checksum drift.

`import-audio` intentionally requires an existing Roblox audio asset ID or manifest entry. It creates
`Sound` instances and applies safe properties, but it does not pretend arbitrary local audio files are
playable without a real `SoundId`.

## Docs

- [Architecture](docs/architecture.md)
- [Protocol](docs/protocol.md)
- [Transfer format](docs/transfer-format.md)
- [Feature specs](docs/feature-specs.md)
- [Autopilot spec](docs/autopilot-spec.md)
- [AI Studio Autopilot wow spec](docs/ai-studio-autopilot-wow-spec.md)

## License

MIT
