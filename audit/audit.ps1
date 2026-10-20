# The audit: the gate a round runs, in ten seconds.
#
# **It tests the change, not the tree.** The repository's rule is "test what you changed": a gate that runs
# every suite every round reports on code the round never touched, and its cost is what makes nobody run it.
# So the stages are:
#
#   [1/4] freshness   -- is every source older than the artefacts it built? (reports; the clock is the judge)
#   [2/4] rustfmt     -- on the files this branch and this working tree changed, not on the world
#   [3/4] the tests   -- the diff, mapped to packages, walked down the *normal* dependency graph, and those
#                        suites run -- nothing else. The closure is printed, so a reader can check the scope.
#   [4/4] the clock   -- **the audit fails if it takes longer than ten seconds**, naming the slowest stage
#
# **What is not audited, and why.** Three stages were removed, each because a person showed it was decoration:
#
# - **the examples are not run one by one.** Sixty-four `cargo run`s bought "compiled and did not panic"; the
#   workspace build already proves the first, and their output is a number a person reads, which nothing here
#   reads. They are compiled by `cargo build --workspace --all-targets`, nothing more.
# - **the scene documents are not hashed.** A document carries the camera, the material table and the
#   composition — **a scene is an aesthetic decision**, and pinning it turns the owner's judgement into a red
#   gate. Seventeen of them also cost four minutes.
# - **the identity probe is not a gate.** `Identity::of` numbers a loop from one, every path is written by a
#   `format!` and one name is one key, so "contiguous, non-empty, both ways" are three things the code cannot
#   do otherwise; and the seventeen-scene row total was a change detector that fired on every legitimate round.
#   `scene_identity_probe` remains the instrument one runs to *print* the table.
#
# The suite that draws frames through the gpu is **never run here**: the gpu belongs to the machine's owner.
# `full-audit.ps1` builds it and leaves it unexecuted, and that file refuses to start without
# `-OnOwnerPermission`.
#
# Language: PowerShell 7. Platform: none of its own -- every path is built with `Join-Path` from
# `$PSScriptRoot`.

$ErrorActionPreference = "Continue"
$clock = [System.Diagnostics.Stopwatch]::StartNew()
$root = Split-Path -Parent $PSScriptRoot
$script:fail = [System.Collections.Generic.List[string]]::new()
$script:stage = [System.Collections.Generic.List[object]]::new()

Set-Location $root

"===================================================================="
"AUDIT  worktree: $root"
"started: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')"
"===================================================================="

# ---------------------------------------------------------------- [1/4]
"--------------------------------------------------------------------"
"[1/4] every source older than the artefact it built"
"--------------------------------------------------------------------"
$target = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $root "target" }
$artefacts = @(Get-ChildItem (Join-Path $target "debug\deps") -Filter *.exe -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending)
if ($artefacts.Count -eq 0) {
    "  no artefacts in $target\debug\deps (nothing has been built here yet)"
} else {
    $newest = $artefacts[0].LastWriteTime
    $sources = @(Get-ChildItem (Join-Path $root "crates") -Recurse -Filter *.rs -ErrorAction SilentlyContinue |
            Where-Object { $_.FullName -notmatch '\\target\\' })
    $stale = @($sources | Where-Object { $_.LastWriteTime -gt $newest })
    "  newest artefact : {0:HH:mm:ss}  {1}" -f $newest, $artefacts[0].Name
    "  sources         : $($sources.Count), $($stale.Count) newer than it"
    # **A warning, not a refusal, because a modification time is not the truth about a build.** A fresh
    # checkout, a rebase or a `git checkout` stamps every file with "now", so a refusal on this signal fires
    # on a tree whose target is perfectly valid. What a stale tree *costs* is [4/4]'s business: a stage that
    # rebuilt is a stage the clock reports, and the audit is the run after it.
    if ($stale.Count -eq 0) {
        "  the tree is warm: no source is newer than the artefacts it built"
    } else {
        "  $($stale.Count) source(s) carry a newer stamp; if a stage below rebuilds, the clock says so"
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
"[3/4] the tests of what the change reaches, and nothing else"
"--------------------------------------------------------------------"
# A path names a **directory**; the graph is keyed by **package**, and `crates/gallery` is `whiting-gallery`.
# Naming the directory in `cargo test -p` is an error cargo reports as "did not match any packages", which this
# stage once printed while calling the tree stale. The mapping is read from the manifests below, so a rename
# cannot leave a stale table here.
$directories = @($changed | ForEach-Object {
        if ($_ -match '^crates/([^/]+)/') { $Matches[1] }
    } | Sort-Object -Unique)

# The workspace root names every crate as an alias: `structure = { path = "crates/structure", package =
# "whiting-structure" }`, and the manifests depend on the alias. Both are read here so the graph is the one
# cargo resolves rather than a second spelling of it.
$aliases = @{}
$rootManifest = Get-Content (Join-Path $root "Cargo.toml") -Raw
foreach ($m in [regex]::Matches($rootManifest, '(?m)^([\w-]+)\s*=\s*\{\s*path\s*=\s*"[^"]+",\s*package\s*=\s*"([^"]+)"')) {
    $aliases[$m.Groups[1].Value] = $m.Groups[2].Value
}
$normal = @{}
$packageOf = @{}
foreach ($dir in (Get-ChildItem (Join-Path $root "crates") -Directory)) {
    $manifest = Join-Path $dir.FullName "Cargo.toml"
    if (-not (Test-Path $manifest)) { continue }
    $text = Get-Content $manifest -Raw
    if ($text -notmatch '(?m)^name\s*=\s*"([^"]+)"') { continue }
    $package = $Matches[1]
    $packageOf[$dir.Name] = $package
    $section = ""
    $plain = [System.Collections.Generic.List[string]]::new()
    foreach ($line in ($text -split "`n")) {
        if ($line -match '^\[(.+)\]') { $section = $Matches[1]; continue }
        if ($line -notmatch '^([\w-]+)\s*=\s*\{') { continue }
        $key = $Matches[1]
        $resolved = if ($aliases.ContainsKey($key)) { $aliases[$key] } elseif ($key -like "whiting-*") { $key } else { $null }
        if (-not $resolved) { continue }
        # Only `[dependencies]`: a dev-dependency is what a crate's own tests reach for, and it does not carry
        # a change down the library graph. Reading both graphs as one was measured to drag eleven crates into
        # the closure of a change in one, which is the "run everything" this stage exists to stop.
        if ($section -eq "dependencies") { $plain.Add($resolved) }
    }
    $normal[$package] = @($plain | Sort-Object -Unique)
}

$touched = @($directories | ForEach-Object { if ($packageOf.ContainsKey($_)) { $packageOf[$_] } else { $_ } } | Sort-Object -Unique)
"  changed .rs files : $($changed.Count)"
"  crates touched    : $(if ($touched.Count -eq 0) { "(none: the diff is empty, and an empty diff has no downstream)" } else { $touched -join ", " })"

$closure = [System.Collections.Generic.List[string]]::new()
$pending = [System.Collections.Generic.Queue[string]]::new()
foreach ($name in $touched) { $pending.Enqueue($name) }
while ($pending.Count -gt 0) {
    $package = $pending.Dequeue()
    if ($closure.Contains($package)) { continue }
    $closure.Add($package)
    foreach ($candidate in $normal.Keys) {
        if ($normal[$candidate] -contains $package) { $pending.Enqueue($candidate) }
    }
}

# The closure is restricted to **this repository's own crates**: the lichen ones live in a sibling checkout,
# their suites are seconds to minutes long, and a change reaching them is a fact about the graph rather than
# something a ten-second audit verifies here -- the lichen crates build against a sibling checkout and their
# suites take tens of seconds. The rasterising crate is excluded because its tests draw frames.
$here = @($packageOf.Values)
$sibling = @("whiting-lichen", "whiting-lichen-link", "whiting-lichen-stages")
$runnable = @($closure | Where-Object { $here -contains $_ -and $sibling -notcontains $_ -and $_ -ne "whiting-device-rasterize" })
"  downstream closure: $(if ($runnable.Count -eq 0) { "(nothing in this repository)" } else { $runnable -join ", " })"
if ($closure -contains "whiting-device-rasterize") {
    "  (whiting-device-rasterize is in the closure and is not run: its tests draw frames)"
}

if ($runnable.Count -eq 0) {
    "  nothing to run: no crate of this repository was touched by this diff"
} else {
    foreach ($package in $runnable) {
        $timer = [System.Diagnostics.Stopwatch]::StartNew()
        # The build's stderr is kept and shown on failure: a stage that says only "did not build" hides the
        # one line that explains it.
        $errorsFile = Join-Path $target "audit-$package.stderr.txt"
        $text = (cargo test -q -p $package -j 3 2>$errorsFile | Out-String)
        $code = $LASTEXITCODE
        $timer.Stop()
        $script:stage.Add([pscustomobject]@{ Name = "test $package"; Seconds = $timer.Elapsed.TotalSeconds })
        $passed = 0
        $failed = 0
        $failedTests = @()
        foreach ($line in ($text -split "`n")) {
            if ($line -match 'test result:.*?(\d+) passed; (\d+) failed') {
                $passed += [int]$Matches[1]
                $failed += [int]$Matches[2]
            }
            if ($line -cmatch '^test .* FAILED') { $failedTests += $line }
        }
        "  {0,-26} passed={1,-4} failed={2,-3}  {3:N2} s" -f $package, $passed, $failed, $timer.Elapsed.TotalSeconds
        foreach ($one in ($failedTests | Select-Object -First 4)) { "      $($one.Trim())" }
        if ($code -ne 0 -and (Test-Path $errorsFile)) {
            Get-Content $errorsFile | Where-Object { $_ -match '^error' } | Select-Object -First 4 | ForEach-Object { "      $($_.Trim())" }
        }
        if ($failed -ne 0) { $script:fail.Add("$package has $failed failing test(s)") }
        if ($code -ne 0) { $script:fail.Add("$package exited $code") }
    }
}

# ---------------------------------------------------------------- [4/4]
"--------------------------------------------------------------------"
"[4/4] the clock: ten seconds, and the stage that spent them"
"--------------------------------------------------------------------"
$clock.Stop()
$slowest = $script:stage | Sort-Object Seconds -Descending | Select-Object -First 1
foreach ($one in ($script:stage | Sort-Object Seconds -Descending)) { "  {0,-26} {1,7:N2} s" -f $one.Name, $one.Seconds }
"  whole audit                {0,7:N2} s   (the budget is 10.00 s)" -f $clock.Elapsed.TotalSeconds
if ($clock.Elapsed.TotalSeconds -gt 10.0) {
    $why = if ($slowest) { $slowest.Name } else { "(no stage ran)" }
    $script:fail.Add(("the audit took {0:N2} s, over its ten-second budget; slowest stage: {1}" -f $clock.Elapsed.TotalSeconds, $why))
}

"--------------------------------------------------------------------"
$verdict = @()
$verdict += "===================================================================="
if ($script:fail.Count -eq 0) {
    $verdict += "AUDIT RESULT: every gate green, in under ten seconds."
} else {
    $verdict += "AUDIT RESULT: $($script:fail.Count) failure(s):"
    foreach ($one in $script:fail) { $verdict += "  - $one" }
}
$verdict += "finished: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')"
$verdict += "===================================================================="
$verdict
if ($script:fail.Count -ne 0) { exit 1 }
exit 0
