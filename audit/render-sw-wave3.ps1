# Renders the six southwest-wave3 plates into one directory and prints each render's own row and
# timing line.
#
# Run with:  pwsh -NoProfile -File audit/render-sw-wave3.ps1 <out-dir>
# from the worktree.
#
# Every path is built from $PSScriptRoot, so the script renders this checkout whichever directory it
# is launched from -- and Set-Location is balanced in a try/finally, because a `Set-Location` left
# standing is how a run ends up compiling the parent checkout into this worktree's target and prints
# a build that looks fine.

$ErrorActionPreference = "Continue"
$root = Split-Path -Parent $PSScriptRoot
$env:CARGO_TARGET_DIR = Join-Path $root "target"
$env:CARGO_INCREMENTAL = "0"

$where = if ($args.Count -ge 1) { $args[0] } else { "out/sw-wave3" }
$out = Join-Path (Join-Path $root "audit") $where
New-Item -ItemType Directory -Force -Path $out | Out-Null

$scenes = @(
    "armand_pine",
    "armand_pine_bough",
    "armand_pine_base",
    "qiong_bamboo",
    "qiong_bamboo_culm",
    "qiong_bamboo_base"
)

Push-Location $root
try {
    "rendering into $out"
    foreach ($scene in $scenes) {
        $stem = Join-Path $out $scene
        $clock = [System.Diagnostics.Stopwatch]::StartNew()
        $run = cargo run -q -j 2 -p whiting-device-rasterize --example render -- $scene $stem 2>&1
        $code = $LASTEXITCODE
        $clock.Stop()
        $run | Where-Object { $_ -match 'drawn,|error|warning' } | ForEach-Object { "  [$_]" }
        "  {0,-18} exit={1} {2:N1}s" -f $scene, $code, $clock.Elapsed.TotalSeconds
    }
}
finally {
    Pop-Location
}
