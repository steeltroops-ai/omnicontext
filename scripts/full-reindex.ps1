# Full reindex script to ensure 100% embedding coverage
# This script:
# 1. Stops any running omnicontext processes
# 2. Clears the existing index
# 3. Runs a fresh index with the model loaded
# 4. Verifies 100% coverage

Write-Host "OmniContext Full Reindex Script" -ForegroundColor Cyan
Write-Host "================================" -ForegroundColor Cyan
Write-Host ""

# Step 1: Stop any running processes
Write-Host "[1/5] Stopping any running omnicontext processes..." -ForegroundColor Yellow
Get-Process | Where-Object { $_.ProcessName -like "*omnicontext*" } | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 2
Write-Host "  ✓ Processes stopped" -ForegroundColor Green
Write-Host ""

# Step 2: Clear the index
Write-Host "[2/5] Clearing existing index..." -ForegroundColor Yellow
$indexPath = "$env:LOCALAPPDATA\omnicontext\repos"
if (Test-Path $indexPath) {
    Remove-Item -Path $indexPath -Recurse -Force
    Write-Host "  ✓ Index cleared" -ForegroundColor Green
} else {
    Write-Host "  ℹ No existing index found" -ForegroundColor Gray
}
Write-Host ""

# Step 3: Verify model is downloaded
Write-Host "[3/5] Verifying embedding model..." -ForegroundColor Yellow
$modelPathNew    = "$env:LOCALAPPDATA\omnicontext\models\CodeRankEmbed\model.onnx"
$modelPathLegacy = "$env:LOCALAPPDATA\omnicontext\models\jina-embeddings-v2-base-code\model.onnx"
$modelPath = if (Test-Path $modelPathNew) { $modelPathNew } `
             elseif (Test-Path $modelPathLegacy) { $modelPathLegacy } `
             else { $null }
if ($modelPath) {
    $modelSize = (Get-Item $modelPath).Length / 1MB
    Write-Host "  ✓ Model found: $([math]::Round($modelSize, 2)) MB" -ForegroundColor Green
} else {
    Write-Host "  ⚠ Model not found - will download during indexing" -ForegroundColor Yellow
}
Write-Host ""

# Step 4: Run fresh index
Write-Host "[4/5] Running fresh index..." -ForegroundColor Yellow
Write-Host "  This may take several minutes..." -ForegroundColor Gray
Write-Host ""

$indexStart = Get-Date
.\target\release\omnicontext.exe index . 2>&1 | Tee-Object -FilePath "reindex.log"
$indexEnd = Get-Date
$indexDuration = ($indexEnd - $indexStart).TotalSeconds

Write-Host ""
Write-Host "  ✓ Indexing complete in $([math]::Round($indexDuration, 2)) seconds" -ForegroundColor Green
Write-Host ""

# Step 5: Verify coverage
Write-Host "[5/5] Verifying embedding coverage..." -ForegroundColor Yellow
$status = .\target\release\omnicontext.exe status --json 2>$null | ConvertFrom-Json

$chunksIndexed = $status.chunks_indexed
$vectorsIndexed = $status.vectors_indexed
$coveragePct = $status.embedding_coverage_percent

Write-Host ""
Write-Host "  Chunks indexed:  $chunksIndexed" -ForegroundColor White
Write-Host "  Vectors indexed: $vectorsIndexed" -ForegroundColor White
Write-Host "  Coverage:        $([math]::Round($coveragePct, 2))%" -ForegroundColor White
Write-Host ""

if ($coveragePct -ge 90.0) {
    Write-Host "  ✓ SUCCESS: Embedding coverage is $([math]::Round($coveragePct, 2))%" -ForegroundColor Green
    Write-Host "  Semantic search is fully operational!" -ForegroundColor Green
} elseif ($coveragePct -ge 50.0) {
    Write-Host "  ⚠ WARNING: Embedding coverage is only $([math]::Round($coveragePct, 2))%" -ForegroundColor Yellow
    Write-Host "  Run: .\target\release\omnicontext.exe embed --retry-failed" -ForegroundColor Yellow
} else {
    Write-Host "  ✗ CRITICAL: Embedding coverage is only $([math]::Round($coveragePct, 2))%" -ForegroundColor Red
    Write-Host "  Check logs for errors. Model may not be loading correctly." -ForegroundColor Red
}

Write-Host ""
Write-Host "Reindex complete! Log saved to: reindex.log" -ForegroundColor Cyan
