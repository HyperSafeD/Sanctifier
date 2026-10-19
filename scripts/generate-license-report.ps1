# scripts/generate-license-report.ps1

$reportPath = "LICENSE_COMPLIANCE_REPORT.md"
$date = Get-Date -Format "yyyy-MM-dd"

$lines = @()
$lines += "# License Compliance Report"
$lines += ""
$lines += "Generated: $date"
$lines += ""
$lines += "## Rust Crates (cargo-deny)"
$lines += ""

if (Test-Path "rust-license-report.txt") {
    $lines += '```'
    $lines += Get-Content "rust-license-report.txt"
    $lines += '```'
} else {
    $lines += "_No cargo-deny output found. Run cargo deny check licenses first._"
}

$workspaces = @("frontend", "vscode-extension", "packages/sanctifier-cli-npm")

foreach ($ws in $workspaces) {
    $lines += ""
    $lines += "## npm workspace: $ws"
    $lines += ""

    $jsonPath = "$ws\license-report.json"
    if (Test-Path $jsonPath) {
        $data = Get-Content $jsonPath -Raw | ConvertFrom-Json
        $lines += "| Package | License |"
        $lines += "|---------|---------|"
        foreach ($prop in $data.PSObject.Properties) {
            $license = $prop.Value.licenses
            $lines += "| $($prop.Name) | $license |"
        }
    } else {
        $lines += "_No license-checker output found for $ws._"
    }
}

$lines | Set-Content -Path $reportPath -Encoding utf8
Write-Host "Report written to $reportPath"