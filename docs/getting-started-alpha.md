# Getting Started Alpha

This private-alpha path keeps the first run narrow: install the Studio plugin, generate a reviewable starter shop plan, and apply it only after the approval packet and live gate say it is safe. Examples use `target\release\rs.exe`; use `rs` instead if the binary is already on your `PATH`.

## Demo 1: Install And Connect Studio

Run from the repository root:

```powershell
cargo build --release -p rs
target\release\rs.exe install-plugin
```

Restart any open Roblox Studio windows so the installed plugin bundle can load, then open the private-alpha place in Studio. Do not stop the bridge or close other sessions just to make this pass.

Check that the CLI can see a compatible Studio session:

```powershell
target\release\rs.exe autopilot setup --timeout 30 --format json
target\release\rs.exe list --json
```

Continue only when `setup` and `list` report the intended Studio session. If they report a stale or missing plugin, rerun `target\release\rs.exe install-plugin`, restart Studio, and check again.

## Demo 2: Generate A Safe Offline Feature Plan

Create the starter shop candidate without touching Studio:

```powershell
target\release\rs.exe autopilot plan "Add a starter shop with coins, two items, and server-side purchase validation" --recipe starterShop --out .rs\autopilot\runs\starter-shop-alpha --format json
target\release\rs.exe autopilot preview --plan .rs\autopilot\runs\starter-shop-alpha\plan.json --format json
target\release\rs.exe autopilot certify .rs\autopilot\runs\starter-shop-alpha --format json
target\release\rs.exe autopilot review-pack .rs\autopilot\runs\starter-shop-alpha --format json
target\release\rs.exe autopilot proof .rs\autopilot\runs\starter-shop-alpha --format json
target\release\rs.exe autopilot approval .rs\autopilot\runs\starter-shop-alpha --format json
```

Review `plan.json`, `preview.json`, `certification.json`, `review-pack.md`,
`proof.json`, and `approval.md` before asking for live approval. `approval.json`
records the plan hash, preview integrity result, and exact apply command. These
files prove only that an offline starter shop candidate exists; they do not
prove it was applied, playtested, published, or production-ready.

## Demo 3: Apply Only After Approval And Close Out Honestly

Use this section only after the creator approves the exact apply command from `approval.md`.

First, run the live gate against the intended Studio session:

```powershell
target\release\rs.exe autopilot live-gate --run-dir .rs\autopilot\runs\starter-shop-alpha --studio "Private Alpha Place" --approved --format json
```

If the gate reports the run is ready for live apply, apply the plan with validation and rollback capture:

```powershell
target\release\rs.exe autopilot apply --studio "Private Alpha Place" --plan .rs\autopilot\runs\starter-shop-alpha\plan.json --yes --validate --rollback-on-error --format json
```

Use the exact command from `live-gate.json`. `autopilot apply` checks
`approval.json`, `live-gate.json`, the current plan hash, preview integrity, and
the selected flags before it contacts Studio.

Record only proof that actually happened. For a real passed Play Solo check, include the observed evidence:

```powershell
target\release\rs.exe autopilot record-playtest .rs\autopilot\runs\starter-shop-alpha --result passed --scenario starterShop=passed --evidence "Play Solo: opened shop, bought an item, and recorded the actual evidence path or observation" --format json
```

If the Play Solo check was not run, failed, or was blocked, use `--result blocked`, `failed`, or `inconclusive` instead and say why in `--note`.

Close out from the artifacts:

```powershell
target\release\rs.exe autopilot closeout .rs\autopilot\runs\starter-shop-alpha --format json
```

Report the closeout verdict as written. A green apply without playtest evidence is still only an applied candidate, not live playtest success or publish readiness.
