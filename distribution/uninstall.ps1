<#
.SYNOPSIS
    OmniContext Uninstaller for Windows

.DESCRIPTION
    Removes OmniContext binaries, PATH entry, indexed data, cached models,
    and unlinks the MCP entry from all known AI client config files.
    Use -KeepData to preserve the vector index and embedding model.
    Use -KeepConfig to leave MCP client configurations untouched.

.PARAMETER KeepData
    Preserve ~/.omnicontext (models, vector indices, repos).

.PARAMETER KeepConfig
    Preserve MCP configuration entries in all AI clients.

.PARAMETER Silent
    Skip the confirmation prompt.

.EXAMPLE
    irm https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/uninstall.ps1 | iex

.EXAMPLE
    .\uninstall.ps1 -KeepData -KeepConfig
#>

param(
    [switch]$KeepData,
    [switch]$KeepConfig,
    [switch]$Silent
)

#Requires -Version 5.1
$ErrorActionPreference = "Stop"

# Enable TLS 1.2 for secure downloads (GitHub)
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

# ---------------------------------------------------------------------------
# helpers
# ---------------------------------------------------------------------------
$ESC = [char]27
function c($code) { "$ESC[${code}m" }

$BOLD   = c "1"; $DIM  = c "2"; $RESET = c "0"
$RED    = c "31"; $GREEN = c "32"; $YELLOW = c "33"
$BLUE   = c "34"; $CYAN  = c "36"

function step   { param($n,$t) Write-Host "$BOLD$CYAN  [$n]$RESET $t" }
function ok     { param($t)    Write-Host "$GREEN  [v]$RESET $t" }
function info   { param($t)    Write-Host "$BLUE  [»]$RESET $t" }
function warn   { param($t)    Write-Host "$YELLOW  [!] $RESET $t" }
function fail   { param($t)    Write-Host "$RED  [x]$RESET $t" }
$HR      = $DIM + ('-' * 54) + $RESET
function hr    { Write-Host $HR }
function blank { Write-Host '' }
function Exit-Err { param($m) blank; fail $m; blank; exit 1 }

$StartTime = Get-Date
$BinDir    = Join-Path $HOME ".omnicontext\bin"
$DataDir   = Join-Path $HOME ".omnicontext"

# ---------------------------------------------------------------------------
# banner
# ---------------------------------------------------------------------------
blank
Write-Host "$BOLD$RED   ____                  _  ______            __            __ $RESET"
Write-Host "$BOLD$RED  / __ \____ ___  ____  (_)/ ____/___  ____  / /____  _  __/ /_$RESET"
Write-Host "$BOLD$RED / / / / __ ``__ \/ __ \/ // /   / __ \/ __ \/ __/ _ \| |/_/ __/$RESET"
Write-Host "$BOLD$RED/ /_/ / / / / / / / / / // /___/ /_/ / / / / /_/  __/_>  </ /_ $RESET"
Write-Host "$BOLD$RED\____/_/ /_/ /_/_/ /_/_/ \____/\____/_/ /_/\__/\___/_/|_|\__/  $RESET"
Write-Host "${DIM}  Universal Code Context Engine -- Uninstaller${RESET}"
hr
blank

# ---------------------------------------------------------------------------
# confirm
# ---------------------------------------------------------------------------
warn "This will remove OmniContext from your system."
if (-not $KeepData)   { warn "Indexed data and AI models (~600 MB+) will be deleted." }
if (-not $KeepConfig) { warn "MCP client configurations will have omnicontext removed." }

blank

if (-not $Silent) {
    $confirm = Read-Host "  Proceed with uninstallation? [y/N]"
    if ($confirm -notmatch "^[yY]") {
        info "Uninstallation cancelled."
        blank
        exit 0
    }
}

blank

# ---------------------------------------------------------------------------
# step 1 – terminate processes
# ---------------------------------------------------------------------------
$procs = Get-Process -Name "omnicontext","omnicontext-mcp","omnicontext-daemon" -EA SilentlyContinue
if ($procs) {
    $procs | Stop-Process -Force -EA SilentlyContinue
    Start-Sleep -Milliseconds 600
    ok "Stopped $($procs.Count) active process(es)"
} else {
    info "No active OmniContext processes found"
}

# Check for .cargo/bin conflicts (Mayank's case)
$CargoBinDir = Join-Path $HOME ".cargo\bin"
$conflicts = @()
if (Test-Path $CargoBinDir) {
    $conflicts = Get-ChildItem $CargoBinDir -Filter "omnicontext*.exe" -EA SilentlyContinue
}

if ($conflicts) {
    blank
    warn "Detected conflicting binaries in .cargo\bin:"
    foreach ($c in $conflicts) { info "  $($c.Name)" }
    if (-not $Silent) {
        $remCargo = Read-Host "  Remove these conflicts too? [y/N]"
        if ($remCargo -match "^[yY]") {
            foreach ($c in $conflicts) {
                Remove-Item $c.FullName -Force -EA SilentlyContinue
                ok "Removed conflict: $($c.Name)"
            }
        }
    }
}

# ---------------------------------------------------------------------------
# step 2 – remove binaries + PATH
# ---------------------------------------------------------------------------
blank
step "2/4" "Removing binaries"

if (Test-Path $BinDir) {
    $exes = Get-ChildItem $BinDir -Filter "omnicontext*.exe" -EA SilentlyContinue
    foreach ($exe in $exes) {
        if (Remove-Item $exe.FullName -Force -EA SilentlyContinue) {
            info "Removed  $DIM$($exe.Name)$RESET"
        }
    }
    Remove-Item $BinDir -Recurse -Force -EA SilentlyContinue
    ok "Binaries uninstalled  $DIM$BinDir$RESET"
} else {
    info "Binary directory not found"
}

# Remove from PATH
$UserPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($UserPath -like "*$BinDir*") {
    $newPath = ($UserPath -split ';' | Where-Object { $_.Trim() -ne "" -and $_.Trim() -ne $BinDir }) -join ';'
    [Environment]::SetEnvironmentVariable("PATH", $newPath, "User")
    ok "Removed from User PATH"
}

# ---------------------------------------------------------------------------
# step 3 – remove data / models
# ---------------------------------------------------------------------------
blank
step "3/4" "Removing data"

if (-not $KeepData) {
    if (Test-Path $DataDir) {
        $sizeMb = [math]::Round(
            (Get-ChildItem $DataDir -Recurse -EA SilentlyContinue | Measure-Object -Property Length -Sum).Sum / 1MB,
            1
        )
        Remove-Item $DataDir -Recurse -Force -EA SilentlyContinue
        ok "Data directory removed  $DIM$DataDir  (${sizeMb} MB freed)$RESET"
    } else {
        info "Data directory not found  $DIM$DataDir$RESET"
    }
} else {
    ok ("Data directory preserved  " + $DIM + "(-KeepData)" + $RESET)
}

# ---------------------------------------------------------------------------
# step 4 – remove MCP from all known clients
# ---------------------------------------------------------------------------
blank
step "4/4" "Unlinking MCP configurations"

if (-not $KeepConfig) {

    # PS 5.1-compatible deep conversion of PSCustomObject → ordered hashtable
    function ConvertTo-Hashtable($obj) {
        if ($obj -is [System.Management.Automation.PSCustomObject]) {
            $ht = [ordered]@{}
            foreach ($prop in $obj.PSObject.Properties) {
                $ht[$prop.Name] = ConvertTo-Hashtable $prop.Value
            }
            return $ht
        }
        return $obj
    }

    function Remove-McpEntry {
        param($Name, $Path, [string]$TopKey)
        if (-not (Test-Path $Path)) { return }
        try {
            $raw = Get-Content $Path -Raw -EA SilentlyContinue
            if (-not $raw) { return }
            $cfg = $raw | ConvertFrom-Json | ForEach-Object { ConvertTo-Hashtable $_ }
            if (-not $cfg) { return }

            $changed = $false
            switch ($TopKey) {
                "powers" {
                    if ($cfg["powers"] -and $cfg["powers"]["mcpServers"] -and $cfg["powers"]["mcpServers"]["omnicontext"]) {
                        $cfg["powers"]["mcpServers"].Remove("omnicontext")
                        $changed = $true
                    }
                }
                "servers" {
                    if ($cfg["servers"] -and $cfg["servers"]["omnicontext"]) {
                        $cfg["servers"].Remove("omnicontext")
                        $changed = $true
                    }
                }
                "context_servers" {
                    if ($cfg["context_servers"] -and $cfg["context_servers"]["omnicontext"]) {
                        $cfg["context_servers"].Remove("omnicontext")
                        $changed = $true
                    }
                }
                default {
                    if ($cfg["mcpServers"] -and $cfg["mcpServers"]["omnicontext"]) {
                        $cfg["mcpServers"].Remove("omnicontext")
                        $changed = $true
                    }
                }
            }

            if ($changed) {
                $cfg | ConvertTo-Json -Depth 10 | Set-Content $Path -Encoding UTF8
                ok "  Unlinked  $DIM$Name$RESET"
            } else {
                info "  Not configured  $DIM$Name$RESET"
            }
        } catch {
            warn "  Could not update  $DIM$Name$RESET  ($($_.Exception.Message))"
        }
    }

    $mcpClients = @(
        @{ Name = "Claude Desktop"; Path = "$env:APPDATA\Claude\claude_desktop_config.json";                                                              TopKey = "mcpServers" },
        @{ Name = "Claude Code CLI";Path = "$env:USERPROFILE\.claude.json";                                                                               TopKey = "mcpServers" },
        @{ Name = "Cursor";         Path = "$env:APPDATA\Cursor\User\mcp.json";                                                                           TopKey = "mcpServers" },
        @{ Name = "Windsurf";       Path = "$env:USERPROFILE\.codeium\windsurf\mcp_config.json";                                                          TopKey = "mcpServers" },
        @{ Name = "VS Code";        Path = "$env:APPDATA\Code\User\mcp.json";                                                                             TopKey = "servers"    },
        @{ Name = "Cline (VS Code)";Path = "$env:APPDATA\Code\User\globalStorage\saoudrizwan.claude-dev\settings\cline_mcp_settings.json";                TopKey = "mcpServers" },
        @{ Name = "RooCode";        Path = "$env:APPDATA\Code\User\globalStorage\rooveterinaryinc.roo-cline\settings\mcp_settings.json";                  TopKey = "mcpServers" },
        @{ Name = "Continue.dev";   Path = "$env:USERPROFILE\.continue\config.json";                                                                      TopKey = "mcpServers" },
        @{ Name = "Kiro";           Path = "$env:USERPROFILE\.kiro\settings\mcp.json";                                                                    TopKey = "powers"     },
        @{ Name = "Trae";           Path = "$env:APPDATA\Trae\User\globalStorage\trae-ide.trae-ai\mcp_settings.json";                                     TopKey = "mcpServers" },
        @{ Name = "Antigravity";    Path = "$env:APPDATA\Antigravity\User\mcp.json";                                                                      TopKey = "servers"    },
        @{ Name = "Gemini CLI";     Path = "$env:USERPROFILE\.gemini\settings.json";                                                                      TopKey = "mcpServers" },
        @{ Name = "Amazon Q CLI";   Path = "$env:USERPROFILE\.aws\amazonq\mcp.json";                                                                      TopKey = "mcpServers" },
        @{ Name = "Augment Code";   Path = "$env:APPDATA\Code\User\globalStorage\augment.vscode-augment\mcp_settings.json";                               TopKey = "mcpServers" },
        @{ Name = "PearAI";         Path = "$env:APPDATA\PearAI\User\mcp.json";                                                                           TopKey = "mcpServers" },
        @{ Name = "Zed";            Path = "$env:APPDATA\Zed\settings.json";                                                                                TopKey = "context_servers" }
    )

    foreach ($client in $mcpClients) {
        Remove-McpEntry -Name $client.Name -Path $client.Path -TopKey $client.TopKey
    }
} else {
    ok ("MCP configurations preserved  " + $DIM + "(-KeepConfig)" + $RESET)
}

# ---------------------------------------------------------------------------
# summary
# ---------------------------------------------------------------------------
$elapsed = [math]::Round(((Get-Date) - $StartTime).TotalSeconds, 1)
blank
hr
Write-Host ("${BOLD}${GREEN}  OmniContext removed${RESET}  " + $DIM + "(${elapsed}s)" + $RESET)
hr
blank

if ($KeepData)   { info "Data preserved at  $DIM$DataDir$RESET" }
if ($KeepConfig) { info "MCP configurations untouched" }

blank
Write-Host "  $DIMTo reinstall: $RESET"
Write-Host "  irm https://raw.githubusercontent.com/steeltroops-ai/omnicontext/main/distribution/install.ps1 | iex"
blank
