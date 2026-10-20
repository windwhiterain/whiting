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
#   [1/4] freshness   -- is every source older than the artefacts it built? (measured: ~0.1 s)
#   [2/4] rustfmt     -- on the files this branch and this working tree changed, not on the world (~0.1 s)
#   [3/4] the suites  -- scene 106, gallery 10, definition 22, graphics 15 (measured warm: 0.3 s to run)
#   [4/4] the clock   -- **the audit fails if it takes longer than ten seconds**, and names the stage that did
#
# **A generated scene is not audited, because the generator is the proof.** The identity table is total by
# construction: `Identity::of` numbers a loop from one, every row's path is written by a `format!`, and a name
# is one key -- so a probe that walks it and reports "contiguous true, every name non-empty, both ways" is
# asserting three things the code cannot do otherwise. Its fourth reading, the seventeen-scene row total, was
# not an invariant but a change detector, and it fired on every legitimate round: adding rafter tails *is*
# changing the table (5 322 rows to 5 670, then to 6 402 when the rule for where a tail goes was corrected).
# `scene_identity_probe` stays as the instrument one runs to *print* the table; it is not a gate.
#
# Nor is a **scene** audited: it is an aesthetic decision, and pinning a camera, a material table or a
# composition would turn the owner's judgement into a red gate. That is why there is no document hash here.
#
# The device-rasterize suite is not run: thirty-eight of its tests draw frames through the gpu, and the gpu
# belongs to the machine's owner. Its binary is built by `full-audit.ps1` and left unexecuted there.
#
# WHAT IS NOT HERE, and where it went
#
# clippy, `cargo doc`, the whole-workspace build, the whole-tree fmt check and the rasterize build are all in
# `full-audit.ps1`. Run that one before a merge that moves a crate, a dependency or an invariant; run this one
# every round.
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

# ---------------------------------------------------------------- [1/4]
"--------------------------------------------------------------------"
"[1/4] every source older than the artefact it built  (a stale tree pays for its own build)"
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
        # **A warning, not a refusal, because a modification time is not the truth about a build.** A fresh
        # checkout, a rebase or a `git checkout` stamps every file with "now", so a refusal on this signal
        # fires on a tree whose target is perfectly valid -- measured: a new worktree over a fully built
        # target refused with all 376 sources "newer". The truth is what carrying a stale tree *costs*, and
        # that is [5/5]'s business: a stage that rebuilt is a stage the clock reports, and the audit is the
        # run after it.
        "  $($stale.Count) source(s) carry a newer stamp than the newest artefact"
        "  (a fresh checkout stamps every file; if a stage below rebuilds, the clock says so and the"
        "   audit is the run after it)"
    } else {
        "  the tree is warm: no source is newer than the artefacts it built"
    }
}

# ---------------------------------------------------------------- [2/4]
"--------------------------------------------------------------------"
"[2/4] rustfmt over the files this branch and this tree changed"
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

# ---------------------------------------------------------------- [3/4]
"--------------------------------------------------------------------"
"[3/4] the four suites that run without a gpu, in the binaries cargo says are current"
"--------------------------------------------------------------------"
# **Cargo is asked where the binaries are, not to run them.** A `cargo test` pays the resolver, the
# fingerprint check and a fresh process for every crate: on a warm tree 0.6-1.2 s each, and now and then one
# of them decides to relink and the stage costs nine seconds -- measured: a suite that runs in 2.0 s took
# 9.24 s, which alone broke the budget. `--no-run --message-format=json` answers in 0.2-0.4 s and *builds
# what is stale*, so the binaries it names are current by cargo's own judgement, and running them directly
# costs hundredths of a second.
$expect = [ordered]@{
    "whiting-scene"      = 106
    "whiting-gallery"    = 10
    "whiting-definition" = 22
    "whiting-graphics"   = 15
}
foreach ($package in $expect.Keys) {
    $timer = [System.Diagnostics.Stopwatch]::StartNew()
    $listed = cargo test -q --no-run --message-format=json -p $package -j 3 2>$null
    if ($LASTEXITCODE -ne 0) {
        $timer.Stop()
        "  {0,-20} COULD NOT BUILD" -f $package
        $script:fail.Add("$package did not build")
        $script:stage.Add([pscustomobject]@{ Name = "test $package"; Seconds = $timer.Elapsed.TotalSeconds })
        continue
    }
    # Only this package's own targets: the message stream carries every dependency that was linked too.
    $binaries = @($listed | ForEach-Object {
            if ("$_" -match '"package_id":"[^"]*' + [regex]::Escape($package) + '[^"]*"' -and
                "$_" -match '"executable":"((?:[^"\\]|\\.)+)"' -and $Matches[1] -ne "null") {
                $Matches[1] -replace '\\\\(.)', '$1'
            }
        } | Sort-Object -Unique)
    $passed = 0
    $failed = 0
    $failedTests = @()
    foreach ($binary in $binaries) {
        if (-not (Test-Path $binary)) { continue }
        $text = & $binary 2>&1
        foreach ($line in $text) {
            if ("$_" -match 'test result:.*?(\d+) passed; (\d+) failed') {
                $passed += [int]$Matches[1]
                $failed += [int]$Matches[2]
            }
        }
        $failedTests += @($text | Where-Object { "$_" -match 'FAILED' })
    }
    $timer.Stop()
    $script:stage.Add([pscustomobject]@{ Name = "test $package"; Seconds = $timer.Elapsed.TotalSeconds })
    $same = $passed -eq $expect[$package]
    "  {0,-20} passed={1,-4} failed={2,-3} expected={3,-4} same={4}  binaries={5}  {6:N2} s" -f $package, $passed, $failed, $expect[$package], $same, $binaries.Count, $timer.Elapsed.TotalSeconds
    foreach ($one in ($failedTests | Select-Object -First 4)) { "      $($one.Trim())" }
    if (-not $same) { $script:fail.Add("$package count ($passed, expected $($expect[$package]))") }
    if ($failed -ne 0) { $script:fail.Add("$package has $failed failing test(s)") }
}

# ---------------------------------------------------------------- [4/4]
"--------------------------------------------------------------------"
"[4/4] the clock: ten seconds, and the stage that spent them"
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