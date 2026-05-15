# Example: transfer the sniper assets from Brainrots into Snipe a Slime.
# Run after building rs and installing the plugin in both Studios.

$ErrorActionPreference = "Stop"

$source = "Snipe for Brainrots!"
$target = "Snipe a Slime!"

$items = @(
    "ServerStorage.SniperSkins",
    "ReplicatedStorage.Miscs",
    "ReplicatedStorage.BulletModel",
    "ReplicatedStorage.GunGUI",
    "ReplicatedStorage.v_SP-R 208",
    "ReplicatedStorage.Sniper1",
    "ReplicatedStorage.Sniper2",
    "ReplicatedStorage.Sniper3"
)

foreach ($item in $items) {
    $parts = $item -split '\.', 2
    $parent = $parts[0]
    rs transfer --from "${source}:${item}" --to "${target}:${parent}"
}

Write-Host "Done. Save the target place in Studio."
