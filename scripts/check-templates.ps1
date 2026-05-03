param(
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
)

$ErrorActionPreference = "Stop"

$templatesRoot = Join-Path $RepoRoot "templates"
$workRoot = Join-Path $RepoRoot "target\template-check"
$jiminyPath = ($RepoRoot -replace '\\', '/')

if (-not (Test-Path $templatesRoot)) {
    throw "templates directory not found: $templatesRoot"
}

Remove-Item -Recurse -Force $workRoot -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $workRoot | Out-Null

Get-ChildItem $templatesRoot -Directory | ForEach-Object {
    $template = $_
    $packageName = "jiminy-template-$($template.Name)"
    $dest = Join-Path $workRoot $template.Name

    Write-Host "== $($template.Name) =="
    Copy-Item -Recurse -Force $template.FullName $dest

    Get-ChildItem $dest -Recurse -File | Where-Object {
        $_.Extension -in @(".toml", ".rs", ".md")
    } | ForEach-Object {
        $text = Get-Content -Raw -Path $_.FullName
        $text = $text -replace '\{\{project-name\}\}', $packageName
        $text = $text -replace 'jiminy = "0\.16"', "jiminy = { path = `"$jiminyPath`" }"
        Set-Content -NoNewline -Path $_.FullName -Value $text
    }

    Add-Content -Path (Join-Path $dest "Cargo.toml") -Value "`n[workspace]`n"

    cargo check --manifest-path (Join-Path $dest "Cargo.toml")
    if ($LASTEXITCODE -ne 0) {
        throw "template check failed for $($template.Name)"
    }
}

Write-Host "OK: all templates compile after placeholder expansion."
