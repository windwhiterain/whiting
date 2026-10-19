# The full audit of the building arc, run as one list on this worktree.
#
# It prints the VALUES it compared, never a bare verdict, because a bare match is how a
# canary reports success on a green log. Every gate keeps its own exit code, read on the
# statement immediately after the command it belongs to.
#
# Run with `pwsh -NoProfile -File audit/full-audit.ps1`, from the worktree.
#
# THE ONE COMMAND:  pwsh -NoProfile -File audit/full-audit.ps1
#
# Language: PowerShell 7 (`pwsh`). Platform: none of its own -- every path is built with
# Join-Path from $PSScriptRoot and no path is spelled with a separator, so the same script
# runs on Linux and macOS as well as Windows. What IS machine-specific is the nine pinned
# hashes: they are a before/after pair ON ONE MACHINE, per "Correction: the difference was
# the hash's domain", so a run on another gpu compares the same nine strings and is expected
# to move them. The gate that travels is the other five, and the script says so here rather
# than letting a red hash on a new machine be read as a broken tree.
#
# WHY A SCRIPT AND NOT A RUST EXAMPLE: an example is itself one of the `--all-targets` this
# file builds and one of the 29/16 examples it counts, so adding the instrument would move a
# number the instrument gates on. The thing being measured is `cargo` -- its exit codes, its
# counts, its stderr -- and the only thing that measures cargo from outside cargo is another
# process.
#
# EXIT POLICY: this script COLLECTS every failure and exits non-zero once, at the end,
# rather than stopping at the first. An audit that reports one red gate per run makes the
# reader pay a full run for every gate they did not know was red, and a full run here is
# dominated by the 45 examples and the nine renders rather than by the build; knowing all of
# them at once is worth that. Nothing is lost by not stopping -- a step that cannot run at
# all (a render that died) is still recorded, and the run after it still happens.

# THE TWO LINT GATES ARE COUNTED, NOT EXIT-CODED. `cargo clippy` and `cargo doc` both exit 0
# on a tree that carries warnings -- that was MEASURED here, by putting one `unused_variables`
# binding and one broken `[`NoSuchThingCanary`]` link into the tree: both runs answered exit 0
# and one warning line each. A gate on the exit code alone would therefore have been a gate
# nobody could watch fire, which is the exact thing this file exists to prevent. Both steps
# print the warning count they counted and compare it against 0, and both were proved to fail
# by putting those two faults in and reading the numbers back: 1 against 0, on runs whose exit
# code was 0.
$ErrorActionPreference = "Continue"
# The worktree root is this script's own parent, so the same script verifies a branch in its
# worktree and a merge on the tip it was merged into. Hard-coding one checkout's path would
# make the second of those runs measure the wrong tree, which is the whole hazard below.
$root = Split-Path -Parent $PSScriptRoot

# `pwsh -File script.ps1` does NOT inherit the tool's workdir: it starts in whatever
# directory the shell was launched from. Cargo then resolves the workspace from the CWD,
# so a run started in the parent checkout compiles the PARENT's sources into whatever
# target directory it is given, and every number it prints looks correct. The first
# version of this script did exactly that and printed the evidence while gating only on
# the exit code. So the directory is set here, asserted here, and every `Compiling`
# line is checked for the worktree -- and that count is a gate, not a print.
Set-Location $root
$here = (Get-Location).Path
"running from: $here"
if ($here -cne $root) { "FATAL: wrong working directory, refusing to run"; exit 2 }

$env:CARGO_TARGET_DIR = Join-Path $root "target"
$out = Join-Path (Join-Path $root "audit") "out"
$log = Join-Path (Join-Path $root "audit") "audit-run.log"
$history = Join-Path (Join-Path $root "audit") "verdicts.log"

$script:fail = @()
function Gate([string]$name, [int]$code, [string]$expected) {
    if ("$code" -ceq "$expected") {
        $line = "  [ok]   {0,-46} exit={1}" -f $name, $code
    } else {
        $line = "  [FAIL] {0,-46} exit={1} (expected {2})" -f $name, $code, $expected
        $script:fail += $name
    }
    $line
    # A verdict kept only on the console cannot be read after the fact: a run that failed once and
    # not again leaves nothing to post-mortem, which is how a transient gate survives unnoticed.
    Add-Content -Path $log -Value $line -Encoding utf8
}

Remove-Item $log -ErrorAction SilentlyContinue

# The nine hashes are read out of a directory this run owns, so it is emptied first and the
# path is checked against the one this script computed before anything is removed. A stale
# `house.png` from a run whose render died after writing is the one input that could make a
# hash pass against a picture this run never drew.
$expectedOut = Join-Path (Join-Path $root "audit") "out"
if ((Split-Path -Parent $out) -cne (Split-Path -Parent $expectedOut) -or $out -cne $expectedOut) {
    "FATAL: the output directory is not the one this script computed ($expectedOut); refusing to remove anything"
    exit 2
}
if (Test-Path $out) { Remove-Item -Recurse -Force $out }
New-Item -ItemType Directory -Force -Path $out | Out-Null
"clean output directory: $out"

"===================================================================="
"AUDIT  worktree: $root"
"started: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')"
"===================================================================="

"--------------------------------------------------------------------"
"[1/8] build --workspace --all-targets, and the worktree discipline"
"--------------------------------------------------------------------"
$build = cargo build --workspace --all-targets -j 3 2>&1
$code = $LASTEXITCODE
$build | Out-File -FilePath $log -Append -Encoding utf8
$compiling = @($build | Where-Object { $_ -match '^\s*Compiling ' })
$mine = @($compiling | Where-Object { $_ -match 'Compiling whiting-' })
$notMine = @($mine | Where-Object { $_ -notmatch [regex]::Escape($root) })
$warn = @($build | Where-Object { $_ -match '^(warning|error)' })
"  Compiling lines total          : $($compiling.Count)"
"  of which whiting-*             : $($mine.Count)"
"  whiting-* NOT naming worktree  : $($notMine.Count)"
$notMine | ForEach-Object { "      $_" }
"  warning/error lines            : $($warn.Count)"
$warn | Select-Object -First 5 | ForEach-Object { "      $_" }
# A gate, not a print: one Compiling line from another checkout means every number below
# was produced by code that is not this branch.
if ($notMine.Count -ne 0) { $script:fail += "a Compiling line names a checkout other than this worktree" }
if ($warn.Count -ne 0) { $script:fail += "the build printed $($warn.Count) warning/error line(s)" }
Gate "cargo build --workspace --all-targets" $code 0

# The same build in release, because a `cfg(debug_assertions)` switch can make code compile in one
# profile and not the other. What this does NOT gate is any rendered number: the nine hashes are
# debug builds, and a release render is a different question -- fma contraction may move a pixel --
# that nobody has asked, so it is not silently assumed to agree.
$release = cargo build --workspace --all-targets --release -j 3 2>&1
$code = $LASTEXITCODE
$release | Out-File -FilePath $log -Append -Encoding utf8
$relCompiling = @($release | Where-Object { $_ -match '^\s*Compiling ' })
$relMine = @($relCompiling | Where-Object { $_ -match 'Compiling whiting-' })
$relNotMine = @($relMine | Where-Object { $_ -notmatch [regex]::Escape($root) })
$relWarn = @($release | Where-Object { $_ -match '^(warning|error)' })
"  release Compiling whiting-*    : $($relMine.Count)"
"  release NOT naming worktree    : $($relNotMine.Count)"
"  release warning/error lines    : $($relWarn.Count)   (the gate compares this against 0)"
$relWarn | Select-Object -First 5 | ForEach-Object { "      $_" }
if ($relNotMine.Count -ne 0) { $script:fail += "a release Compiling line names a checkout other than this worktree" }
if ($relWarn.Count -ne 0) { $script:fail += "the release build printed $($relWarn.Count) warning/error line(s)" }
Gate "cargo build --workspace --all-targets --release" $code 0

"--------------------------------------------------------------------"
"[2/8] cargo clippy over the three building crates: warning count against 0"
"--------------------------------------------------------------------"
# The exit code is printed beside the count and is NOT the gate. Both readings are taken because
# they are two claims -- "clippy ran" and "clippy had nothing to say" -- and the first is true of a
# tree full of warnings. The count is the one that decides.
$clippy = cargo clippy -q -p whiting-building -p whiting-gallery -p whiting-scene --all-targets -j 3 2>&1
$code = $LASTEXITCODE
$clippy | Out-File -FilePath $log -Append -Encoding utf8
# Anchored at column 0 and case-sensitive, with the `:` or `[` the compiler always writes after the
# kind: a code span, a note and a `help:` line cannot match it, and the run prints no
# "N warnings emitted" summary of its own. The same form counted 137 on the tree this gate was
# added to and 1 on a tree carrying a single `unused_variables` binding.
$lintLines = @($clippy | Where-Object { "$_" -cmatch '^(warning|error)(\[|:)' })
"  exit code                       : $code"
"  warning/error lines counted     : $($lintLines.Count)   (the gate compares this against 0)"
$lintLines | Select-Object -First 10 | ForEach-Object { "      $_" }
if ($lintLines.Count -ne 0) { $script:fail += "clippy printed $($lintLines.Count) warning/error line(s) over the three crates" }
Gate "cargo clippy -p building -p gallery -p scene" $code 0

"--------------------------------------------------------------------"
"[3/8] cargo doc -p whiting-building --no-deps: warning count against 0"
"--------------------------------------------------------------------"
# The same two readings and the same reason. `--no-deps` because this is this crate's own
# documentation and the count a reader of docs.rs would see; rustdoc checks only public
# documentation by default, so the number is the count of the links a reader cannot follow.
$doc = cargo doc -q -p whiting-building --no-deps 2>&1
$code = $LASTEXITCODE
$doc | Out-File -FilePath $log -Append -Encoding utf8
$docLines = @($doc | Where-Object { "$_" -cmatch '^(warning|error)(\[|:)' })
"  exit code                       : $code"
"  warning/error lines counted     : $($docLines.Count)   (the gate compares this against 0)"
$docLines | Select-Object -First 10 | ForEach-Object { "      $_" }
if ($docLines.Count -ne 0) { $script:fail += "cargo doc printed $($docLines.Count) warning/error line(s) for whiting-building" }
Gate "cargo doc -p whiting-building --no-deps" $code 0

"--------------------------------------------------------------------"
"[4/8] the six test suites"
"--------------------------------------------------------------------"
# The two the audit list never ran -- definition and graphics -- are in this table with their
# counts, because the debt sweep found them compiled by the build and executed by nobody:
# a suite whose count is printed and not compared is decoration, which is the one thing this
# file exists to not be.
$expect = [ordered]@{
    "whiting-scene"           = 111
    "whiting-gallery"         = 10
    "whiting-device-cycles"   = 3
    "whiting-device-rasterize" = 49
    "whiting-definition"      = 22
    "whiting-graphics"        = 15
}
foreach ($pkg in $expect.Keys) {
    $t = cargo test -p $pkg -j 3 2>&1
    $code = $LASTEXITCODE
    $t | Out-File -FilePath $log -Append -Encoding utf8
    # `test result:` is cargo's own line; sum the numbers rather than matching on "failed",
    # which also occurs inside "0 failed".
    $passed = 0; $failed = 0; $ignored = 0
    foreach ($line in $t) {
        if ($line -match 'test result:.*?(\d+) passed; (\d+) failed; (\d+) ignored') {
            $passed += [int]$Matches[1]; $failed += [int]$Matches[2]; $ignored += [int]$Matches[3]
        }
    }
    # The exact form, case-SENSITIVE, counted and compared on its own: a case-insensitive
    # `-match 'FAILED'` reports fifteen failures on a fully green suite because it also reads
    # "0 failed". Two readings of one log, so neither can quietly stop reading.
    $failedLines = @($t | Where-Object { "$_" -cmatch 'test result: FAILED' }).Count
    "  {0,-24} passed={1,-4} failed={2,-3} ignored={3,-3} 'test result: FAILED'={4,-3} expected={5}" -f $pkg, $passed, $failed, $ignored, $failedLines, $expect[$pkg]
    if ($failed -ne 0) { $script:fail += "$pkg has $failed failing test(s)" }
    if ($failedLines -ne 0) { $script:fail += "$pkg printed $failedLines 'test result: FAILED' line(s)" }
    if ($passed -ne $expect[$pkg]) { $script:fail += "$pkg count (passed $passed, the audit says $($expect[$pkg]))" }
    Gate "cargo test -p $pkg" $code 0
}

"--------------------------------------------------------------------"
"[5/8] every example in building and gallery"
"--------------------------------------------------------------------"
foreach ($pair in @(@("whiting-building", "building", 37), @("whiting-gallery", "gallery", 17))) {
    $pkg = $pair[0]; $dir = $pair[1]; $want = $pair[2]
    $examples = Get-ChildItem (Join-Path (Join-Path (Join-Path $root "crates") $dir) "examples") -Filter *.rs | Sort-Object Name
    $ran = 0; $bad = @()
    foreach ($ex in $examples) {
        $name = $ex.BaseName
        $e = cargo run -q -p $pkg --example $name 2>&1
        $code = $LASTEXITCODE
        $e | Out-File -FilePath $log -Append -Encoding utf8
        $ran++
        if ($code -ne 0) { $bad += "$name(exit=$code)" }
    }
    "  {0,-18} ran={1} expected={2} failures={3}" -f $dir, $ran, $want, $bad.Count
    $bad | ForEach-Object { "      FAILED $_" }
    if ($ran -ne $want) { $script:fail += "$dir example count ($ran, expected $want)" }
    if ($bad.Count -ne 0) { $script:fail += "$dir failing examples" }
}

"--------------------------------------------------------------------"
"[6/8] the twelve written .png hashes  (sha256 of the file, first 16 hex upper)"
"--------------------------------------------------------------------"
$pinned = [ordered]@{
    house      = "4A8795039E713F32"; hall       = "8B6495E3A9B2FE89"; hall_large = "CD2FA9166AACB06C"
    shed       = "C3A11B4465746840"; terrace    = "D2E405C7B242E92B"; wing       = "B4516134C2EC5D6E"
    workshop   = "7FE07F345A451B51"; yard       = "F36F4CB6A14312FA"; identity   = "717B5570DDAD7A0D"
    stilt      = "03510DF10AD236E4"; compound   = "FA4AD3009BD581AC"
    street     = "BD46F1ACB223C05F"
}
# **The eleven above moved together on 2026-10-19, and it is recorded here rather than in a commit
# message nobody reads beside the number.** Causes, each measured by a verification pass that rendered
# the plates at four commits and diffed them pixel by pixel:
#
# - `f5ed72d` halved a point light's shadow page (`projection_of` went from `2 * half * padding` to
#   `half * padding`), so every point-lit surface's shadow lookup changed. That is what moved the eleven:
#   1.2 % to 7.2 % of pixels, maximum magnitude 26..103 of 255 and not one pixel above 128, on the mass,
#   its cast shadow and the ground line under it, with the sky and the roofs reading zero.
# - The emitter rewrite (`2508818`, `5213583`, `507a29e`, `ee7f258`) moved **no pixel at all**: the two
#   plates rendered at the rung that carries all of it are byte-identical to the baseline, and the
#   emitted document's own hash is unchanged. `4398d41`'s `ShadowBias` changed the shader *text* and no
#   pixel, because `f32 * 1.0` is exact for every building schema.
# - No building scene module changed between the green commit and this one, and the identity probe's
#   rows are the same numbers (9 280 over eleven scenes, 14 220 with the street).
$pinned
# How it is proved to fail: change one pinned value above, run it, watch the two numbers
# disagree and the exit code come back 1, then change it back. That is the whole test, and it
# was run that way rather than by a switch inside this file: an earlier version carried a
# `-Canary` switch, and the first full run of it found that PowerShell variable names are
# case-insensitive, so `$canary = if ($Canary) {...}` was assigning a string back into the
# switch parameter and printing "canary: False" on a run that had armed it. A self-test whose
# own message variable collides with its own parameter is decoration wearing a gate's name.
$moved = 0
$rendered = 0
foreach ($scene in $pinned.Keys) {
    $stem = Join-Path $out $scene
    $r = cargo run -q -p whiting-device-rasterize --example render -- $scene $stem 2>&1
    $code = $LASTEXITCODE
    $r | Out-File -FilePath $log -Append -Encoding utf8
    $png = "$stem.png"
    $rendered++
    if ($code -ne 0 -or -not (Test-Path $png)) { "  $scene  RENDER FAILED exit=$code file=$([bool](Test-Path $png))"; $script:fail += "$scene render (exit $code)"; $moved++; continue }
    $got = (Get-FileHash -Algorithm SHA256 $png).Hash.Substring(0, 16)
    $want = $pinned[$scene]
    $same = $got -ceq $want
    "  {0,-11} got {1}  pinned {2}  same={3}" -f $scene, $got, $want, $same
    if (-not $same) { $script:fail += "$scene hash: got $got, pinned $want"; $moved++ }
}
"  scenes rendered: $rendered of $($pinned.Count)   hashes moved: $moved of $($pinned.Count)"

"--------------------------------------------------------------------"
"[7/8] scene_identity_probe: rows and two-way"
"--------------------------------------------------------------------"
$p = cargo run -q -p whiting-gallery --example scene_identity_probe 2>&1
$code = $LASTEXITCODE
$p | Out-File -FilePath $log -Append -Encoding utf8
$rows = 0
foreach ($line in $p) { if ($line -match '^\s+\S+\s+(\d+) row\(s\), ids 1\.\.=(\d+)') { $rows += [int]$Matches[1] } }
$twoWay = @($p | Where-Object { $_ -match 'the channel is two-way for every scene: (\w+)' })
$twoWayValue = if ($twoWay.Count -gt 0) { ($twoWay[0] -split ': ')[-1] } else { "(line absent)" }
"  rows summed over scenes : $rows   (audit says 13620)"
"  two-way line reads      : $twoWayValue"
if ($rows -ne 13620) { $script:fail += "identity rows ($rows, expected 13620)" }
if ($twoWayValue -cne "true") { $script:fail += "identity channel not two-way" }
Gate "scene_identity_probe" $code 0

"--------------------------------------------------------------------"
"[8/8] cargo fmt --all -- --check"
"--------------------------------------------------------------------"
# Both readings, because they are two claims: exit 0 says "rustfmt is satisfied", and an
# empty output says "it printed no diff". A rustfmt that exits 0 while saying something is a
# different instrument from one that exits 0 silently, and the second number is what says so.
$fmt = @(cargo fmt --all -- --check 2>&1)
$code = $LASTEXITCODE
$fmt | Out-File -FilePath $log -Append -Encoding utf8
"  exit code : $code      output lines : $($fmt.Count)"
$fmt | Select-Object -First 10 | ForEach-Object { "      $_" }
Gate "cargo fmt --all -- --check" $code 0
if ($fmt.Count -ne 0) { $script:fail += "cargo fmt --check printed $($fmt.Count) line(s)" }

"===================================================================="
$verdict = @()
if ($script:fail.Count -eq 0) {
    $verdict += "AUDIT RESULT: every gate green."
} else {
    $verdict += "AUDIT RESULT: $($script:fail.Count) failure(s):"
    $verdict += $script:fail | ForEach-Object { "  - $_" }
}
$verdict += "finished: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')"
$verdict += "full output: $log"
$verdict += "===================================================================="
$verdict
Add-Content -Path $log -Value $verdict -Encoding utf8
Add-Content -Path $history -Value ((($verdict -join " ") -replace "=+","") + "  [" + (Get-Date -Format "yyyy-MM-dd HH:mm:ss") + "]") -Encoding utf8
if ($script:fail.Count -ne 0) { exit 1 }
exit 0
