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

# **The gpu is not this audit's to use, and no frame is compared.** The machine runs other work on the
# gpu, so nothing here drives one; and a picture is not what this file holds still — the scenes are
# pinned by the **documents** they compile to, which are renderer-independent and deterministic.
"===================================================================="
"AUDIT  worktree: $root"
"started: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')"
"===================================================================="

# Returns the warning and error lines a build printed about **this workspace's own crates**.
#
# A path dependency under a sibling checkout (the lichen stages are one) compiles into this build, and
# its warnings appear in this output. They are not this workspace's warnings and cannot be fixed here,
# so the gate must not read them as its own. Each warning is attributed by the `--> ` line that follows
# it, which is where cargo names the file; a warning with no such line is kept, because a gate that
# silently drops what it cannot place is worse than one that goes red.
function Get-OwnWarnings {
    param($Output, $Root)
    $kept = @()
    for ($i = 0; $i -lt $Output.Count; $i++) {
        if ($Output[$i] -notmatch '^(warning|error)') { continue }
        # A `generated N warnings` summary repeats what the individual lines already said, and cargo
        # prints it for every crate in the build, dependency or not.
        if ($Output[$i] -match 'generated \d+ warning') { continue }
        $where = $null
        for ($j = $i + 1; $j -lt [Math]::Min($Output.Count, $i + 6); $j++) {
            if ($Output[$j] -match '^\s+--> (.+)$') { $where = $Matches[1]; break }
            if ($Output[$j] -match '^(warning|error)') { break }
        }
        if ($null -eq $where -or $where.StartsWith($Root, [System.StringComparison]::OrdinalIgnoreCase)) {
            $kept += $Output[$i]
        }
    }
    return $kept
}

"--------------------------------------------------------------------"
"[1/8] build --workspace --all-targets, and the worktree discipline"
"--------------------------------------------------------------------"
$build = cargo build --workspace --all-targets -j 3 2>&1
$code = $LASTEXITCODE
$build | Out-File -FilePath $log -Append -Encoding utf8
$compiling = @($build | Where-Object { $_ -match '^\s*Compiling ' })
$mine = @($compiling | Where-Object { $_ -match 'Compiling whiting-' })
$notMine = @($mine | Where-Object { $_ -notmatch [regex]::Escape($root) })
$warn = @(Get-OwnWarnings $build $root)
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
$relWarn = @(Get-OwnWarnings $release $root)
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
$lintLines = @(Get-OwnWarnings $clippy $root)
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
    "whiting-scene"           = 106
    "whiting-gallery"         = 10
    "whiting-device-cycles"   = 0
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

# **`whiting-device-rasterize` is compiled and not run.** Thirty-eight of its tests draw frames through
# wgpu, and the gpu on this machine is the owner's: the rule is that no frame is drawn through it. The
# suite is therefore built here and its tests are left unexecuted, which is a real loss of coverage and
# is recorded as one. Neither is any frame compared: see [6/8], which reads documents.
$t = cargo test -q -p whiting-device-rasterize --no-run 2>&1
$code = $LASTEXITCODE
$t | Out-File -FilePath $log -Append -Encoding utf8
$counted = @($t | Select-Object -Skip 0) | Out-Null
"  whiting-device-rasterize built, not run (38 of its tests draw frames; the gpu is not ours)"
Gate "cargo test -p whiting-device-rasterize --no-run" $code 0

"--------------------------------------------------------------------"
"[5/8] every example in building and gallery"
"--------------------------------------------------------------------"
foreach ($pair in @(@("whiting-building", "building", 40), @("whiting-gallery", "gallery", 24))) {
    $pkg = $pair[0]; $dir = $pair[1]; $want = $pair[2]
    $examples = Get-ChildItem (Join-Path (Join-Path (Join-Path $root "crates") $dir) "examples") -Filter *.rs | Sort-Object Name
    # **An example that declares a usage line is a harness and needs arguments**, so it is counted and
    # not run: `scene_document` writes a scene to a path the caller names, and running it bare exits 1 by
    # design. The list is read from the source rather than written here, so a new harness is skipped the
    # day it is added and a new *test* example is not.
    $harnesses = @($examples | Where-Object {
            (Get-Content $_.FullName) -match 'usage: '
        } | ForEach-Object { $_.BaseName })
    $ran = 0; $bad = @(); $skipped = @()
    foreach ($ex in $examples) {
        $name = $ex.BaseName
        $ran++
        if ($harnesses -contains $name) { $skipped += $name; continue }
        $e = cargo run -q -p $pkg --example $name 2>&1
        $code = $LASTEXITCODE
        $e | Out-File -FilePath $log -Append -Encoding utf8
        if ($code -ne 0) { $bad += "$name(exit=$code)" }
    }
    "  {0,-18} ran={1} expected={2} failures={3} harnesses={4}" -f $dir, $ran, $want, $bad.Count, ($skipped -join ",")
    $bad | ForEach-Object { "      FAILED $_" }
    if ($ran -ne $want) { $script:fail += "$dir example count ($ran, expected $want)" }
    if ($bad.Count -ne 0) { $script:fail += "$dir failing examples" }
}

"--------------------------------------------------------------------"
"[6/8] the seventeen scene documents: hashes of the compiled documents"
"--------------------------------------------------------------------"
# **No gpu, and no picture.** The rasterising harness drives one, and the machine this audit runs on
# has other work for it. The gate below is the **document** the renderer reads rather than the pixels
# it makes: a document is renderer-independent and deterministic (measured: two writes of one scene
# are the same 6.7 MB and the same hash), so this pins what the scenes *are* without spending a gpu
# — or a comparison of images — on what they look like.
$pinned = [ordered]@{
    house      = "1F648E1FB44D0D73"; hall       = "18F9EE14235A31F4"; hall_large = "276B44CC8C82691F"
    shed       = "E7457703620E9085"; terrace    = "23541F590BC93856"; wing       = "4E57123C48D74B4B"
    workshop   = "046ADB591CABC007"; yard       = "5DF1EBB9AED91D54"; identity   = "4298E2960A48285D"
    stilt      = "86F74C0B66348F56"; compound   = "6AAD1D2CD464B03C"; compound_court = "7C99C4BDA8890F27"
    stilt_gallery = "B8EDEBDC592E2FDE"
    street     = "07E1337D733A0725"; street_arcade = "0AAED8AD9C1EE823"
    yikeyin    = "273D958F73EE63AD"; yikeyin_court = "681FB58AB7AE8A29"
}
# How it is proved to fail: change one pinned value, run the stage, watch the numbers disagree and the
# exit code come back 1, then change it back.
$moved = 0
$written = 0
foreach ($scene in $pinned.Keys) {
    $document = Join-Path $out "$scene.json"
    $r = cargo run -q -p whiting-gallery --example scene_document -- $scene $document 2>&1
    $code = $LASTEXITCODE
    $r | Out-File -FilePath $log -Append -Encoding utf8
    $written++
    if ($code -ne 0 -or -not (Test-Path $document)) {
        "  $scene  DOCUMENT FAILED exit=$code"; $script:fail += "$scene document (exit $code)"; $moved++
        continue
    }
    $got = (Get-FileHash -Algorithm SHA256 $document).Hash.Substring(0, 16)
    $want = $pinned[$scene]
    $same = $got -ceq $want
    "  {0,-14} got {1}  pinned {2}  same={3}" -f $scene, $got, $want, $same
    if (-not $same) { $script:fail += "$scene document: got $got, pinned $want"; $moved++ }
}
"  documents written: $written of $($pinned.Count)   hashes moved: $moved of $($pinned.Count)"


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
"  rows summed over scenes : $rows   (audit says 48356)"
"  two-way line reads      : $twoWayValue"
if ($rows -ne 48356) { $script:fail += "identity rows ($rows, expected 48356)" }
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
