# The full audit of the building arc, run as one list on this worktree.
#
# It prints the VALUES it compared, never a bare verdict, because a bare match is how a
# canary reports success on a green log. Every gate keeps its own exit code, read on the
# statement immediately after the command it belongs to.
#
# Run with `pwsh -NoProfile -File audit\full-audit.ps1`, from the worktree.

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

$env:CARGO_TARGET_DIR = "$root\target"
$out = "$root\audit\out"
$log = "$root\audit\audit-run.log"

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
New-Item -ItemType Directory -Force -Path $out | Out-Null

"===================================================================="
"AUDIT  worktree: $root"
"started: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')"
"===================================================================="

"--------------------------------------------------------------------"
"[1/7] build --workspace --all-targets, and the worktree discipline"
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
"[2/7] the four test suites"
"--------------------------------------------------------------------"
$expect = @{ "whiting-scene" = 111; "whiting-gallery" = 10; "whiting-device-cycles" = 3; "whiting-device-rasterize" = 49 }
foreach ($pkg in @("whiting-scene", "whiting-gallery", "whiting-device-cycles", "whiting-device-rasterize")) {
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
    "  {0,-24} passed={1,-4} failed={2,-3} ignored={3,-3} (audit says {4})" -f $pkg, $passed, $failed, $ignored, $expect[$pkg]
    if ($failed -ne 0) { $script:fail += "$pkg tests" }
    if ($passed -ne $expect[$pkg]) { $script:fail += "$pkg count (got $passed, audit says $($expect[$pkg]))" }
    Gate "cargo test -p $pkg" $code 0
}

"--------------------------------------------------------------------"
"[3/7] every example in building and gallery"
"--------------------------------------------------------------------"
foreach ($pair in @(@("whiting-building", "building", 29), @("whiting-gallery", "gallery", 16))) {
    $pkg = $pair[0]; $dir = $pair[1]; $want = $pair[2]
    $examples = Get-ChildItem "$root\crates\$dir\examples" -Filter *.rs | Sort-Object Name
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
"[4/7] the nine written .png hashes  (sha256 of the file, first 16 hex upper)"
"--------------------------------------------------------------------"
$pinned = [ordered]@{
    house      = "2F60685F737F7ED7"; hall       = "96F61FA5E9F18E60"; hall_large = "478E692C6CBE2D3B"
    shed       = "48909F693ACBA634"; terrace    = "E32C1CA08D9F3016"; wing       = "2EF5C4FB206167F4"
    workshop   = "EB8E05CD755B9B4B"; yard       = "D33D5F3372A4F0B0"; identity   = "48F13F7D84A24480"
}
$moved = 0
foreach ($scene in $pinned.Keys) {
    $r = cargo run -q -p whiting-device-rasterize --example render -- $scene "$out\$scene" 2>&1
    $code = $LASTEXITCODE
    $r | Out-File -FilePath $log -Append -Encoding utf8
    $png = "$out\$scene.png"
    if ($code -ne 0 -or -not (Test-Path $png)) { "  $scene  RENDER FAILED exit=$code"; $script:fail += "$scene render"; $moved++; continue }
    $got = (Get-FileHash -Algorithm SHA256 $png).Hash.Substring(0, 16)
    $want = $pinned[$scene]
    $same = $got -ceq $want
    "  {0,-11} got {1}  pinned {2}  same={3}" -f $scene, $got, $want, $same
    if (-not $same) { $script:fail += "$scene hash moved"; $moved++ }
}
"  scenes rendered: $($pinned.Count)   hashes moved: $moved"

"--------------------------------------------------------------------"
"[5/7] scene_identity_probe: rows and two-way"
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
"[6/7] cargo fmt --all -- --check"
"--------------------------------------------------------------------"
cargo fmt --all -- --check 2>&1 | Out-File -FilePath $log -Append -Encoding utf8
$code = $LASTEXITCODE
Gate "cargo fmt --all -- --check" $code 0

"--------------------------------------------------------------------"
"[7/7] the extra sweep: crates whose tests no audit gate runs"
"--------------------------------------------------------------------"
"  whiting-definition and whiting-graphics have test files the audit list never runs:"
"  they are compiled by the build above and executed here only."
foreach ($pkg in @("whiting-definition", "whiting-graphics")) {
    $t = cargo test -p $pkg -j 3 2>&1
    $code = $LASTEXITCODE
    $t | Out-File -FilePath $log -Append -Encoding utf8
    $passed = 0; $failed = 0
    foreach ($line in $t) {
        if ($line -match 'test result:.*?(\d+) passed; (\d+) failed') { $passed += [int]$Matches[1]; $failed += [int]$Matches[2] }
    }
    "  {0,-22} passed={1,-4} failed={2}" -f $pkg, $passed, $failed
    if ($failed -ne 0) { $script:fail += "$pkg tests" }
}

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
