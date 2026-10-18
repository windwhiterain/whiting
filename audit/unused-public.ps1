# Debt sweep: for every `pub fn` / `pub const` in the four crates this arc touched,
# count the call sites across the whole tree, and separate the ones with no caller
# outside their own file from the ones nobody calls at all.
#
# Analysis only. It writes a report; it never edits Rust.

$root = "C:\resource\whiting\.worktrees\building-debt"
$crates = @("building", "gallery", "scene", "device-cycles")

$files = Get-ChildItem -Path $root -Recurse -Filter *.rs |
    Where-Object { $_.FullName -notmatch '\\target\\' }

# one big string per file, so a match is counted once per file per name.
# Doc comments are stripped first: a name written in prose with a bracket after it is a
# mention, not a call, and counting it would hide exactly the leftovers this looks for.
$content = @{}
foreach ($f in $files) {
    $code = (Get-Content $f.FullName) |
        Where-Object { $_ -notmatch '^\s*(///|//!|\*|/\*)' } |
        ForEach-Object { $_ -replace '//.*$', '' }
    $content[$f.FullName] = ($code -join "`n")
}

$rows = @()
foreach ($crate in $crates) {
    $crateDir = Join-Path $root "crates\$crate"
    Get-ChildItem -Path $crateDir -Recurse -Filter *.rs | ForEach-Object {
        $src = $_.FullName
        $lines = Get-Content $src
        for ($i = 0; $i -lt $lines.Count; $i++) {
            $line = $lines[$i]
            if ($line -match '^\s*pub(?:\([^)]*\))?\s+(?:async\s+)?fn\s+([a-z_][a-z0-9_]*)') {
                $name = $Matches[1]
                if ($name -in @("main", "new", "default", "clone", "drop", "from")) { continue }
                $declFile = $src
                $declLine = $i + 1
                $outside = @()
                $inside = 0
                foreach ($path in $content.Keys) {
                    $n = ([regex]::Matches($content[$path], "\b$([regex]::Escape($name))\s*\(")).Count
                    if ($n -eq 0) { continue }
                    if ($path -eq $declFile) { $inside = $n } else { $outside += [pscustomobject]@{ File = $path.Substring($root.Length + 1); Hits = $n } }
                }
                $rows += [pscustomobject]@{
                    Crate   = $crate
                    Name    = $name
                    Decl    = $declFile.Substring($root.Length + 1) + ":" + $declLine
                    Outside = ($outside | ForEach-Object { "$($_.File)($($_.Hits))" }) -join "; "
                    # `$null` when there is no outside caller at all, and `$null -ne 0` -- so the
                    # count is spelled with an explicit default rather than left to Compare-Object.
                    OutsideCount = [int](($outside | Measure-Object -Property Hits -Sum).Sum ?? 0)
                    # hits in the declaring file itself, which includes the declaration
                    OwnFileHits = ([regex]::Matches($content[$declFile], "\b$([regex]::Escape($name))\s*\(")).Count
                }
            }
        }
    }
}

"=== A. called NOWHERE at all: declaration only, no call in any file (own file hits = 1) ==="
$nowhere = $rows | Where-Object { $_.OutsideCount -eq 0 -and $_.OwnFileHits -le 1 }
$nowhere | Sort-Object Crate, Decl | ForEach-Object { "{0,-13} {1,-28} {2}" -f $_.Crate, $_.Name, $_.Decl }
"--- count A: $($nowhere.Count) ---"

"=== B. no caller outside its own file, but called inside it ==="
$inside = $rows | Where-Object { $_.OutsideCount -eq 0 -and $_.OwnFileHits -gt 1 }
$inside | Sort-Object Crate, Decl | ForEach-Object { "{0,-13} {1,-28} {2}  (own-file calls: {3})" -f $_.Crate, $_.Name, $_.Decl, ($_.OwnFileHits - 1) }
"--- count B: $($inside.Count) ---"

"=== counts ==="
"pub fn scanned: $($rows.Count)"
"no caller in any other file: $(($rows | Where-Object { $_.OutsideCount -eq 0 }).Count)"

"=== canary: `lowest` has exactly one caller and it is an example, so it must NOT appear in A or B ==="
$rows | Where-Object { $_.Name -eq 'lowest' } | ForEach-Object { "lowest -> outside=$($_.OutsideCount) own=$($_.OwnFileHits) :: $($_.Outside)" }
