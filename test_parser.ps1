$content = Get-Content -Raw "distribution\install.ps1"
$errors = $null
$ast = [System.Management.Automation.Language.Parser]::ParseInput($content, [ref]$null, [ref]$errors)
if ($errors) {
    foreach ($e in $errors) {
        Write-Host "Line $($e.Extent.StartLineNumber) Char $($e.Extent.StartColumnNumber): $($e.Message)"
    }
} else {
    Write-Host "No AST errors!"
}
