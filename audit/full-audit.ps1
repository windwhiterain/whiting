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

$script:fail = @()
function Gate([string]$name, [int]$code, [string]$expected) {
    if ("$code" -ceq "$expected") {
        "  [ok]   {0,-46} exit={1}" -f $name, $code
    } else {
        "  [FAIL] {0,-46} exit={1} (expected {2})" -f $name, $code, $expected
        $script:fail += $name
    }
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
"[1/6] build --workspace --all-targets, and the worktree discipline"
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

"--------------------------------------------------------------------"
"[2/6] the six test suites"
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
"[3/6] every example in building and gallery"
"--------------------------------------------------------------------"
foreach ($pair in @(@("whiting-building", "building", 29), @("whiting-gallery", "gallery", 16))) {
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
"[4/6] the nine written .png hashes  (sha256 of the file, first 16 hex upper)"
"--------------------------------------------------------------------"
$pinned = [ordered]@{
    house      = "2F60685F737F7ED7"; hall       = "96F61FA5E9F18E60"; hall_large = "478E692C6CBE2D3B"
    shed       = "48909F693ACBA634"; terrace    = "E32C1CA08D9F3016"; wing       = "2EF5C4FB206167F4"
    workshop   = "EB8E05CD755B9B4B"; yard       = "D33D5F3372A4F0B0"; identity   = "48F13F7D84A24480"
}
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
"[5/6] scene_identity_probe: rows and two-way"
"--------------------------------------------------------------------"
$p = cargo run -q -p whiting-gallery --example scene_identity_probe 2>&1
$code = $LASTEXITCODE
$p | Out-File -FilePath $log -Append -Encoding utf8
$rows = 0
foreach ($line in $p) { if ($line -match '^\s+\S+\s+(\d+) row\(s\), ids 1\.\.=(\d+)') { $rows += [int]$Matches[1] } }
$twoWay = @($p | Where-Object { $_ -match 'the channel is two-way for every scene: (\w+)' })
$twoWayValue = if ($twoWay.Count -gt 0) { ($twoWay[0] -split ': ')[-1] } else { "(line absent)" }
"  rows summed over scenes : $rows   (audit says 856)"
"  two-way line reads      : $twoWayValue"
if ($rows -ne 856) { $script:fail += "identity rows ($rows, expected 856)" }
if ($twoWayValue -cne "true") { $script:fail += "identity channel not two-way" }
Gate "scene_identity_probe" $code 0

"--------------------------------------------------------------------"
"[6/6] cargo fmt --all -- --check"
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
if ($script:fail.Count -eq 0) {
    "AUDIT RESULT: every gate green."
} else {
    "AUDIT RESULT: $($script:fail.Count) failure(s):"
    $script:fail | ForEach-Object { "  - $_" }
}
"finished: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')"
"full output: $log"
"===================================================================="
if ($script:fail.Count -ne 0) { exit 1 }
exit 0
