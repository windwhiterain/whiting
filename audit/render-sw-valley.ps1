# Renders the seven southwest-valley plates into one directory, and prints each render's own
# row, pass and timing line.
#
# Run with:  pwsh -NoProfile -File audit/render-sw-valley.ps1 <out-dir>
# from the worktree.
#
# The directory defaults to audit/out/after. It is under audit/out, which .gitignore holds, so a
# render never lands on the working tree.
#
# Every path is built from $PSScriptRoot, so the script renders this checkout whichever directory
# it is launched from -- and Set-Location is balanced in a try/finally, because a `Set-Location`
# left standing is how a run ends up compiling the parent checkout into this worktree's target and
# prints a build that looks fine.

$ErrorActionPreference = "Continue"
$root = Split-Path -Parent $PSScriptRoot
$env:CARGO_TARGET_DIR = Join-Path $root "target"
$env:CARGO_INCREMENTAL = "0"

$where = if ($args.Count -ge 1) { $args[0] } else { "out/after" }
$out = Join-Path (Join-Path $root "audit") $where
New-Item -ItemType Directory -Force -Path $out | Out-Null

$scenes = @(
    "tree_fern",
    "tree_fern_frond",
    "tree_fern_base",
    "kapok",
    "kapok_flower",
    "kapok_bough",
    "kapok_base"
)

Push-Location $root
try {
    "rendering into $out"
    foreach ($scene in $scenes) {
        $stem = Join-Path $out $scene
        $run = cargo run -q -j 2 -p whiting-device-rasterize --example render -- $scene $stem 2>&1
        $code = $LASTEXITCODE
        $run | Where-Object { $_ -match 'drawn,|error|note' } | ForEach-Object { "  [$_]" }
        "  {0,-16} exit={1}" -f $scene, $code
    }
}
finally {
    Pop-Location
}
