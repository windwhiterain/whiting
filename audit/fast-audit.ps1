# The fast audit: the whole gate in **ten seconds**, run every round.
#
# WHY THIS FILE EXISTS
#
# `full-audit.ps1` grew a stage per arc, and the cost of it is dominated by things that are not gates but
# builds: `cargo fmt --all -- --check` walks every crate (8 s), `cargo check --workspace --all-targets`
# re-checks the world (20 s warm), and the two stages that *matter* -- sixty-four examples, seventeen scene
# documents -- each pay a process launch and a scene build, which is where the ten minutes went. A gate that
# costs ten minutes is a gate nobody runs, so the default is this file and the deep pass is opt-in.
#
# What is kept here is what a round can actually break and what is cheap *when the tree is warm*:
#
#   [1/5] freshness   -- is every source older than the artefacts it built? (measured: ~0.1 s)
#   [2/5] rustfmt     -- on the files this branch and this working tree changed, not on the world (~0.1 s)
#   [3/5] the suites  -- scene 106, gallery 10, definition 22, graphics 15 (measured warm: 5.1 s together)
#   [4/5] the probe   -- seventeen scenes, 54 332 identity rows, both directions (measured: 0.55 s)
#   [5/5] the clock   -- **the audit fails if it takes longer than ten seconds**, and names the stage that did
#
# The device-rasterize suite is not run: thirty-eight of its tests draw frames through the gpu, and the gpu
# belongs to the machine's owner. Its binary is built by `full-audit.ps1` and left unexecuted there.
#
# WHAT IS NOT HERE, and where it went
#
# clippy, `cargo doc`, the sixty-four examples, the seventeen document hashes, the whole-tree fmt check and
# the rasterize build are all in `full-audit.ps1`. Run that one before a merge that moves a plate, a pin or a
# dependency; run this one every round.
#
# Language: PowerShell 7. Platform: none of its own -- every path is built with `Join-Path` from
# `$PSScriptRoot`.

$ErrorActionPreference = "Stop"
$clock = [System.Diagnostics.Stopwatch]::StartNew()
$root = Split-Path -Parent $PSScriptRoot
$script:fail = [System.Collections.Generic.List[string]]::new()
$script:stage = [System.Collections.Generic.List[object]]::new()

Set-Location $root

function Gate([string]$name, [int]$code, [int]$expected) {
    $verdict = if ($code -eq $expected) { "[ok]  " } else { "[FAIL]"; $script:fail.Add("$name (exit $code, expected $expected)") }
    "  {0} {1} exit={2}" -f $verdict, $name, $code
}

"===================================================================="
"AUDIT (fast)  worktree: $root"
"started: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')"
"===================================================================="

# ---------------------------------------------------------------- [1/5]
"--------------------------------------------------------------------"
"[1/5] every source older than the artefact it built  (a stale tree pays for its own build)"
"--------------------------------------------------------------------"
$target = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $root "target" }
$artefacts = @(Get-ChildItem (Join-Path $target "debug\deps") -Filter *.exe -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending)
if ($artefacts.Count -eq 0) {
    "  no artefacts in $target\debug\deps: build once before the fast audit can mean anything"
    $script:fail.Add("no build artefacts")
} else {
    $newest = $artefacts[0].LastWriteTime
    $sources = @(Get-ChildItem (Join-Path $root "crates") -Recurse -Filter *.rs -ErrorAction SilentlyContinue |
            Where-Object { $_.FullName -notmatch '\\target\\' })
    $stale = @($sources | Where-Object { $_.LastWriteTime -gt $newest })
    "  newest artefact : {0:HH:mm:ss}  {1}" -f $newest, $artefacts[0].Name
    "  sources         : $($sources.Count), $($stale.Count) newer than it"
    foreach ($one in ($stale | Select-Object -First 5)) { "      stale: $($one.FullName.Substring($root.Length + 1))" }
    if ($stale.Count -ne 0) {
        # Not a failure on its own -- cargo will rebuild inside [3/5] and the clock in [5/5] will say what
        # that cost. Saying it here is what makes a ten-minute run explain itself.
        "  the suites below will rebuild $($stale.Count) file(s); a cold tree cannot be audited in ten seconds"
    }
}

# ---------------------------------------------------------------- [2/5]
"--------------------------------------------------------------------"
"[2/5] rustfmt over the files this branch and this tree changed"
"--------------------------------------------------------------------"
$changed = @()
$changed += @(git -C $root diff --name-only main -- '*.rs' 2>$null)
$changed += @(git -C $root diff --name-only -- '*.rs' 2>$null)
$changed += @(git -C $root diff --cached --name-only -- '*.rs' 2>$null)
$changed = @($changed | Where-Object { $_ } | Sort-Object -Unique)
$unformatted = @()
foreach ($file in $changed) {
    $full = Join-Path $root $file
    if (-not (Test-Path $full)) { continue }
    & rustfmt --edition 2024 --check $full 2>&1 | Out-Null
    if ($LASTEXITCODE -ne 0) { $unformatted += $file }
}
"  changed .rs files: $($changed.Count)   unformatted: $($unformatted.Count)"
foreach ($file in $unformatted) { "      $file" }
if ($unformatted.Count -ne 0) { $script:fail.Add("$($unformatted.Count) unformatted file(s)") }

# ---------------------------------------------------------------- [3/5]
"--------------------------------------------------------------------"
"[3/5] the four suites that run without a gpu"
"--------------------------------------------------------------------"
$expect = [ordered]@{
    "whiting-scene"      = 106
    "whiting-gallery"    = 10
    "whiting-definition" = 22
    "whiting-graphics"   = 15
}
foreach ($package in $expect.Keys) {
    $timer = [System.Diagnostics.Stopwatch]::StartNew()
    $text = cargo test -q -p $package -j 3 2>&1
    $code = $LASTEXITCODE
    $timer.Stop()
    $script:stage.Add([pscustomobject]@{ Name = "test $package"; Seconds = $timer.Elapsed.TotalSeconds })
    $passed = 0
    $failed = 0
    foreach ($line in $text) {
        if ($line -match 'test result:.*?(\d+) passed; (\d+) failed') {
            $passed += [int]$Matches[1]
            $failed += [int]$Matches[2]
        }
    }
    $same = $passed -eq $expect[$package]
    "  {0,-20} passed={1,-4} failed={2,-3} expected={3,-4} same={4}  {5:N2} s" -f $package, $passed, $failed, $expect[$package], $same, $timer.Elapsed.TotalSeconds
    if (-not $same) { $script:fail.Add("$package count ($passed, expected $($expect[$package]))") }
    if ($failed -ne 0) { $script:fail.Add("$package has $failed failing test(s)") }
    Gate "cargo test -p $package" $code 0
}

# ---------------------------------------------------------------- [4/5]
"--------------------------------------------------------------------"
"[4/5] scene_identity_probe: seventeen scenes, every row, both ways"
"--------------------------------------------------------------------"
$timer = [System.Diagnostics.Stopwatch]::StartNew()
$probe = cargo run -q -p whiting-gallery --example scene_identity_probe 2>&1
$code = $LASTEXITCODE
$timer.Stop()
$script:stage.Add([pscustomobject]@{ Name = "identity probe"; Seconds = $timer.Elapsed.TotalSeconds })
$rows = 0
$scenes = 0
foreach ($line in $probe) {
    if ($line -match '^\s+(\S+)\s+(\d+) row\(s\)') { $rows += [int]$Matches[2]; $scenes++ }
}
$two_way = @($probe | Where-Object { "$_" -match 'two-way for every scene:\s+true' }).Count -gt 0
"  scenes={0}  rows={1}  two-way={2}  (the audit says 17 and 54332)  {3:N2} s" -f $scenes, $rows, $two_way, $timer.Elapsed.TotalSeconds
if ($scenes -ne 17) { $script:fail.Add("probe scenes ($scenes, expected 17)") }
if ($rows -ne 54332) { $script:fail.Add("probe rows ($rows, expected 54332)") }
if (-not $two_way) { $script:fail.Add("the probe's table is not two-way") }
Gate "scene_identity_probe" $code 0

# ---------------------------------------------------------------- [5/5]
"--------------------------------------------------------------------"
"[5/5] the clock: ten seconds, and the stage that spent them"
"--------------------------------------------------------------------"
$clock.Stop()
$slowest = $script:stage | Sort-Object Seconds -Descending | Select-Object -First 1
foreach ($one in ($script:stage | Sort-Object Seconds -Descending)) { "  {0,-24} {1,7:N2} s" -f $one.Name, $one.Seconds }
"  whole audit            {0,7:N2} s   (the budget is 10.00 s)" -f $clock.Elapsed.TotalSeconds
if ($clock.Elapsed.TotalSeconds -gt 10.0) {
    $script:fail.Add(("the audit took {0:N2} s, over its ten-second budget; slowest stage: {1}" -f $clock.Elapsed.TotalSeconds, $slowest.Name))
}

"--------------------------------------------------------------------"
$verdict = @()
$verdict += "===================================================================="
if ($script:fail.Count -eq 0) {
    $verdict += "AUDIT (fast) RESULT: every gate green, in under ten seconds."
} else {
    $verdict += "AUDIT (fast) RESULT: $($script:fail.Count) failure(s):"
    foreach ($one in $script:fail) { $verdict += "  - $one" }
}
$verdict += "finished: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')"
$verdict += "===================================================================="
$verdict
if ($script:fail.Count -ne 0) { exit 1 }
exit 0
