# Persist Development Environment Setup Script for Windows
# Cross-platform tool validation and installation for Windows PowerShell
# Supports: Windows 10/11 with Chocolatey, winget, or Scoop
# Usage: .\setup-dev-environment.ps1 [OPTIONS]

param(
    [switch]$AutoInstall,
    [switch]$Verbose,
    [switch]$DryRun,
    [switch]$Help
)

# Color support for Windows Terminal
$Global:ColorSupport = $false
if ($env:WT_SESSION -or $env:ConEmuANSI -eq "ON" -or $env:TERM_PROGRAM) {
    $Global:ColorSupport = $true
}

# Tool definitions
$Global:Tools = @{
    "rustc" = @{ 
        Type = "required"
        Description = "Rust compiler"
        InstallCommand = "winget install Rustlang.Rustup"
        CheckCommand = "rustc --version"
    }
    "cargo" = @{ 
        Type = "required"
        Description = "Rust package manager"
        InstallCommand = "winget install Rustlang.Rustup"
        CheckCommand = "cargo --version"
    }
    "rustfmt" = @{ 
        Type = "recommended"
        Description = "Rust code formatter"
        InstallCommand = "rustup component add rustfmt"
        CheckCommand = "rustfmt --version"
    }
    "clippy" = @{ 
        Type = "recommended"
        Description = "Rust linter"
        InstallCommand = "rustup component add clippy"
        CheckCommand = "cargo clippy --version"
    }
    "python" = @{ 
        Type = "required"
        Description = "Python interpreter (3.8+)"
        InstallCommand = "winget install Python.Python.3.11"
        CheckCommand = "python --version"
    }
    "pip" = @{ 
        Type = "required"
        Description = "Python package installer"
        InstallCommand = "python -m ensurepip --upgrade"
        CheckCommand = "pip --version"
    }
    "maturin" = @{ 
        Type = "recommended"
        Description = "Python extension builder"
        InstallCommand = "pip install maturin"
        CheckCommand = "maturin --version"
    }
    "black" = @{ 
        Type = "recommended"
        Description = "Python code formatter"
        InstallCommand = "pip install black"
        CheckCommand = "black --version"
    }
    "ruff" = @{ 
        Type = "recommended"
        Description = "Python linter"
        InstallCommand = "pip install ruff"
        CheckCommand = "ruff --version"
    }
    "mypy" = @{ 
        Type = "optional"
        Description = "Python type checker"
        InstallCommand = "pip install mypy"
        CheckCommand = "mypy --version"
    }
    "pytest" = @{ 
        Type = "recommended"
        Description = "Python testing framework"
        InstallCommand = "pip install pytest pytest-cov"
        CheckCommand = "pytest --version"
    }
    "git" = @{ 
        Type = "required"
        Description = "Version control system"
        InstallCommand = "winget install Git.Git"
        CheckCommand = "git --version"
    }
    "cmake" = @{ 
        Type = "optional"
        Description = "Cross-platform build system"
        InstallCommand = "winget install Kitware.CMake"
        CheckCommand = "cmake --version"
    }
}

# Counters
$Global:InstalledCount = 0
$Global:MissingCount = 0

# Print functions
function Write-Header {
    param([string]$Message)
    Write-Host ""
    if ($Global:ColorSupport) {
        Write-Host "====================================================" -ForegroundColor Magenta
        Write-Host $Message -ForegroundColor Magenta
        Write-Host "====================================================" -ForegroundColor Magenta
    } else {
        Write-Host "===================================================="
        Write-Host $Message
        Write-Host "===================================================="
    }
    Write-Host ""
}

function Write-Info {
    param([string]$Message)
    if ($Global:ColorSupport) {
        Write-Host "[INFO] " -ForegroundColor Blue -NoNewline
        Write-Host $Message
    } else {
        Write-Host "[INFO] $Message"
    }
}

function Write-Success {
    param([string]$Message)
    if ($Global:ColorSupport) {
        Write-Host "[SUCCESS] " -ForegroundColor Green -NoNewline
        Write-Host $Message
    } else {
        Write-Host "[SUCCESS] $Message"
    }
}

function Write-Warning {
    param([string]$Message)
    if ($Global:ColorSupport) {
        Write-Host "[WARNING] " -ForegroundColor Yellow -NoNewline
        Write-Host $Message
    } else {
        Write-Host "[WARNING] $Message"
    }
}

function Write-Error {
    param([string]$Message)
    if ($Global:ColorSupport) {
        Write-Host "[ERROR] " -ForegroundColor Red -NoNewline
        Write-Host $Message
    } else {
        Write-Host "[ERROR] $Message"
    }
}

function Write-Verbose {
    param([string]$Message)
    if ($Verbose) {
        if ($Global:ColorSupport) {
            Write-Host "[VERBOSE] " -ForegroundColor Cyan -NoNewline
            Write-Host $Message
        } else {
            Write-Host "[VERBOSE] $Message"
        }
    }
}

# Detect package managers
function Get-PackageManager {
    $packageManagers = @()
    
    # Check for winget (Windows Package Manager)
    try {
        winget --version | Out-Null
        $packageManagers += "winget"
        Write-Verbose "Found winget"
    } catch {
        Write-Verbose "winget not found"
    }
    
    # Check for Chocolatey
    try {
        choco --version | Out-Null
        $packageManagers += "chocolatey"
        Write-Verbose "Found Chocolatey"
    } catch {
        Write-Verbose "Chocolatey not found"
    }
    
    # Check for Scoop
    try {
        scoop --version | Out-Null
        $packageManagers += "scoop"
        Write-Verbose "Found Scoop"
    } catch {
        Write-Verbose "Scoop not found"
    }
    
    return $packageManagers
}

# Check if a command exists
function Test-Command {
    param([string]$CommandName)
    
    try {
        $null = Get-Command $CommandName -ErrorAction Stop
        return $true
    } catch {
        return $false
    }
}

# Get version of a tool
function Get-ToolVersion {
    param([string]$Tool)
    
    try {
        switch ($Tool) {
            "rustc" { 
                $output = rustc --version 2>$null
                return ($output -split ' ')[1]
            }
            "cargo" { 
                $output = cargo --version 2>$null
                return ($output -split ' ')[1]
            }
            "python" { 
                $output = python --version 2>$null
                return ($output -split ' ')[1]
            }
            "pip" { 
                $output = pip --version 2>$null
                return ($output -split ' ')[1]
            }
            "git" { 
                $output = git --version 2>$null
                return ($output -split ' ')[2]
            }
            default {
                if (Test-Command $Tool) {
                    try {
                        $output = & $Tool --version 2>$null | Select-Object -First 1
                        if ($output -match '(\d+\.\d+(?:\.\d+)?)') {
                            return $matches[1]
                        } else {
                            return "installed"
                        }
                    } catch {
                        return "installed"
                    }
                } else {
                    return "not found"
                }
            }
        }
    } catch {
        return "not found"
    }
}

# Check Python version compatibility
function Test-PythonVersion {
    try {
        $versionOutput = python -c "import sys; print(f'{sys.version_info.major}.{sys.version_info.minor}')" 2>$null
        $major, $minor = $versionOutput -split '\.'
        
        if ([int]$major -ge 3 -and [int]$minor -ge 8) {
            return $true
        } else {
            Write-Warning "Python $versionOutput found, but 3.8+ is required"
            return $false
        }
    } catch {
        return $false
    }
}

# Install a tool
function Install-Tool {
    param(
        [string]$ToolName,
        [array]$PackageManagers
    )
    
    $tool = $Global:Tools[$ToolName]
    Write-Info "Installing $ToolName ($($tool.Description))..."
    
    if ($DryRun) {
        Write-Info "[DRY RUN] Would install $ToolName using: $($tool.InstallCommand)"
        return $true
    }
    
    try {
        switch ($ToolName) {
            "rustc" {
                if ("winget" -in $PackageManagers) {
                    Invoke-Expression "winget install Rustlang.Rustup --silent"
                } elseif ("chocolatey" -in $PackageManagers) {
                    Invoke-Expression "choco install rustup.install -y"
                } elseif ("scoop" -in $PackageManagers) {
                    Invoke-Expression "scoop install rustup"
                } else {
                    Write-Error "No supported package manager found for Rust installation"
                    return $false
                }
                
                # Refresh environment variables
                $env:PATH = [System.Environment]::GetEnvironmentVariable("PATH", "Machine") + ";" + [System.Environment]::GetEnvironmentVariable("PATH", "User")
            }
            "cargo" {
                # Usually installed with rustc
                if (Test-Command "rustup") {
                    Write-Info "Cargo already available with Rust toolchain"
                } else {
                    return (Install-Tool "rustc" $PackageManagers)
                }
            }
            "rustfmt" {
                if (Test-Command "rustup") {
                    Invoke-Expression "rustup component add rustfmt"
                } else {
                    Write-Error "rustup not found. Install Rust first."
                    return $false
                }
            }
            "clippy" {
                if (Test-Command "rustup") {
                    Invoke-Expression "rustup component add clippy"
                } else {
                    Write-Error "rustup not found. Install Rust first."
                    return $false
                }
            }
            "python" {
                if ("winget" -in $PackageManagers) {
                    Invoke-Expression "winget install Python.Python.3.11 --silent"
                } elseif ("chocolatey" -in $PackageManagers) {
                    Invoke-Expression "choco install python3 -y"
                } elseif ("scoop" -in $PackageManagers) {
                    Invoke-Expression "scoop install python"
                } else {
                    Write-Error "No supported package manager found for Python installation"
                    Write-Info "Please install Python manually from https://python.org"
                    return $false
                }
                
                # Refresh environment variables
                $env:PATH = [System.Environment]::GetEnvironmentVariable("PATH", "Machine") + ";" + [System.Environment]::GetEnvironmentVariable("PATH", "User")
            }
            "pip" {
                if (Test-Command "python") {
                    Invoke-Expression "python -m ensurepip --upgrade"
                } else {
                    Write-Error "python not found. Install Python first."
                    return $false
                }
            }
            "git" {
                if ("winget" -in $PackageManagers) {
                    Invoke-Expression "winget install Git.Git --silent"
                } elseif ("chocolatey" -in $PackageManagers) {
                    Invoke-Expression "choco install git -y"
                } elseif ("scoop" -in $PackageManagers) {
                    Invoke-Expression "scoop install git"
                } else {
                    Write-Error "No supported package manager found for Git installation"
                    return $false
                }
            }
            "cmake" {
                if ("winget" -in $PackageManagers) {
                    Invoke-Expression "winget install Kitware.CMake --silent"
                } elseif ("chocolatey" -in $PackageManagers) {
                    Invoke-Expression "choco install cmake -y"
                } elseif ("scoop" -in $PackageManagers) {
                    Invoke-Expression "scoop install cmake"
                } else {
                    Write-Error "No supported package manager found for CMake installation"
                    return $false
                }
            }
            default {
                # Python packages
                if ($ToolName -in @("maturin", "black", "ruff", "mypy", "pytest")) {
                    if (Test-Command "pip") {
                        $installCmd = $tool.InstallCommand
                        Invoke-Expression $installCmd
                    } else {
                        Write-Error "pip not found. Install Python and pip first."
                        return $false
                    }
                } else {
                    Write-Warning "Don't know how to install $ToolName automatically"
                    return $false
                }
            }
        }
        
        Write-Success "$ToolName installed successfully"
        return $true
    } catch {
        Write-Error "Failed to install $ToolName`: $($_.Exception.Message)"
        return $false
    }
}

# Check all tools
function Test-Tools {
    Write-Header "Checking Development Tools"
    
    $installedTools = @()
    $missingRequired = @()
    $missingRecommended = @()
    $missingOptional = @()
    
    foreach ($toolName in $Global:Tools.Keys) {
        $tool = $Global:Tools[$toolName]
        $version = Get-ToolVersion $toolName
        
        if (Test-Command $toolName) {
            # Special check for Python version
            if ($toolName -eq "python" -and -not (Test-PythonVersion)) {
                $missingRequired += $toolName
                Write-Error "✗ $toolName ($version) - $($tool.Description) - VERSION TOO OLD"
            } else {
                $installedTools += $toolName
                Write-Success "✓ $toolName ($version) - $($tool.Description)"
                $Global:InstalledCount++
            }
        } else {
            switch ($tool.Type) {
                "required" {
                    $missingRequired += $toolName
                    Write-Error "✗ $toolName - $($tool.Description) [REQUIRED]"
                }
                "recommended" {
                    $missingRecommended += $toolName
                    Write-Warning "✗ $toolName - $($tool.Description) [RECOMMENDED]"
                }
                "optional" {
                    $missingOptional += $toolName
                    Write-Info "✗ $toolName - $($tool.Description) [OPTIONAL]"
                }
            }
            $Global:MissingCount++
        }
    }
    
    # Summary
    Write-Host ""
    if ($installedTools.Count -gt 0) {
        Write-Success "✅ Installed Tools ($($installedTools.Count)):"
        foreach ($tool in $installedTools) {
            Write-Host "  ✅ $tool"
        }
    }
    
    if ($missingRequired.Count -gt 0) {
        Write-Host ""
        Write-Error "❌ Missing Required Tools ($($missingRequired.Count)):"
        foreach ($tool in $missingRequired) {
            $toolInfo = $Global:Tools[$tool]
            Write-Host "  ❌ $tool - $($toolInfo.Description)"
            Write-Host "     Install: $($toolInfo.InstallCommand)"
        }
    }
    
    if ($missingRecommended.Count -gt 0) {
        Write-Host ""
        Write-Warning "⚠️  Missing Recommended Tools ($($missingRecommended.Count)):"
        foreach ($tool in $missingRecommended) {
            $toolInfo = $Global:Tools[$tool]
            Write-Host "  ⚠️  $tool - $($toolInfo.Description)"
            Write-Host "     Install: $($toolInfo.InstallCommand)"
        }
    }
    
    if ($missingOptional.Count -gt 0) {
        Write-Host ""
        Write-Info "ℹ️  Missing Optional Tools ($($missingOptional.Count)):"
        foreach ($tool in $missingOptional) {
            $toolInfo = $Global:Tools[$tool]
            Write-Host "  ℹ️  $tool - $($toolInfo.Description)"
            Write-Host "     Install: $($toolInfo.InstallCommand)"
        }
    }
    
    # Auto-install if requested
    if ($AutoInstall) {
        $allMissing = $missingRequired + $missingRecommended
        
        if ($allMissing.Count -gt 0) {
            Write-Header "Auto-Installing Missing Tools"
            
            $packageManagers = Get-PackageManager
            Write-Info "Available package managers: $($packageManagers -join ', ')"
            
            if ($packageManagers.Count -eq 0) {
                Write-Warning "No supported package managers found. Please install winget, Chocolatey, or Scoop first."
                Show-PackageManagerInstallation
                return
            }
            
            foreach ($tool in $allMissing) {
                Install-Tool $tool $packageManagers
            }
            
            Write-Header "Re-checking Tools After Installation"
            Test-ToolsFinal
        } else {
            Write-Success "All required and recommended tools are already installed!"
        }
    }
}

# Final check after installation
function Test-ToolsFinal {
    $successCount = 0
    $totalCount = 0
    
    foreach ($toolName in $Global:Tools.Keys) {
        $tool = $Global:Tools[$toolName]
        if ($tool.Type -eq "required" -or $tool.Type -eq "recommended") {
            $totalCount++
            if (Test-Command $toolName) {
                if ($toolName -eq "python" -and -not (Test-PythonVersion)) {
                    continue
                }
                $successCount++
                Write-Success "✓ $toolName"
            } else {
                Write-Error "✗ $toolName"
            }
        }
    }
    
    Write-Host ""
    if ($successCount -eq $totalCount) {
        Write-Success "🎉 All required and recommended tools are now installed!"
        Write-Info "You can now run: .\build-and-test.sh (in Git Bash) or use PowerShell equivalents"
    } else {
        Write-Warning "Some tools are still missing ($successCount/$totalCount installed)"
        Write-Info "You may need to restart your terminal or refresh environment variables"
    }
}

# Show package manager installation guide
function Show-PackageManagerInstallation {
    Write-Header "Package Manager Installation"
    
    Write-Info "To use auto-installation, you need a package manager. Choose one:"
    Write-Host ""
    
    Write-Info "1. winget (Windows Package Manager) - Recommended for Windows 10/11:"
    Write-Host "   Already included in recent Windows versions"
    Write-Host "   If missing, install from Microsoft Store: 'App Installer'"
    Write-Host ""
    
    Write-Info "2. Chocolatey - Popular third-party package manager:"
    Write-Host "   Run as Administrator:"
    Write-Host "   Set-ExecutionPolicy Bypass -Scope Process -Force"
    Write-Host "   [System.Net.ServicePointManager]::SecurityProtocol = [System.Net.ServicePointManager]::SecurityProtocol -bor 3072"
    Write-Host "   iex ((New-Object System.Net.WebClient).DownloadString('https://community.chocolatey.org/install.ps1'))"
    Write-Host ""
    
    Write-Info "3. Scoop - User-space package manager:"
    Write-Host "   Set-ExecutionPolicy RemoteSigned -Scope CurrentUser"
    Write-Host "   irm get.scoop.sh | iex"
    Write-Host ""
}

# Show installation guide
function Show-InstallationGuide {
    Write-Header "Manual Installation Guide"
    
    Write-Info "Windows Installation Commands:"
    Write-Host ""
    
    Write-Info "Using winget (recommended):"
    Write-Host "   winget install Git.Git"
    Write-Host "   winget install Python.Python.3.11"
    Write-Host "   winget install Rustlang.Rustup"
    Write-Host "   winget install Kitware.CMake"
    Write-Host ""
    
    Write-Info "Using Chocolatey:"
    Write-Host "   choco install git python3 rustup.install cmake -y"
    Write-Host ""
    
    Write-Info "After installing base tools:"
    Write-Host "   rustup component add rustfmt clippy"
    Write-Host "   pip install maturin black ruff mypy pytest pytest-cov"
    Write-Host ""
    
    Write-Info "Alternative manual downloads:"
    Write-Host "   Git: https://git-scm.com/download/win"
    Write-Host "   Python: https://python.org/downloads/"
    Write-Host "   Rust: https://rustup.rs/"
    Write-Host "   CMake: https://cmake.org/download/"
    Write-Host ""
}

# Show usage information
function Show-Usage {
    Write-Host @"
Persist Development Environment Setup Script for Windows

USAGE:
    .\setup-dev-environment.ps1 [OPTIONS]

DESCRIPTION:
    This script checks for required development tools and optionally installs them.
    Supports Windows 10/11 with winget, Chocolatey, or Scoop package managers.

OPTIONS:
    -AutoInstall    Automatically install missing tools
    -Verbose        Enable verbose output
    -DryRun         Show what would be done without executing
    -Help           Show this help message

EXAMPLES:
    # Check what tools are installed/missing
    .\setup-dev-environment.ps1

    # Automatically install missing tools
    .\setup-dev-environment.ps1 -AutoInstall

    # See what would be installed without making changes
    .\setup-dev-environment.ps1 -AutoInstall -DryRun

    # Get detailed output during installation
    .\setup-dev-environment.ps1 -AutoInstall -Verbose

NOTE:
    You may need to run: Set-ExecutionPolicy RemoteSigned -Scope CurrentUser
    to allow PowerShell script execution.

"@
}

# Main function
function Main {
    if ($Help) {
        Show-Usage
        return
    }
    
    Write-Header "Persist Development Environment Setup"
    
    $packageManagers = Get-PackageManager
    Write-Info "Platform: Windows"
    Write-Info "Available package managers: $(if ($packageManagers.Count -gt 0) { $packageManagers -join ', ' } else { 'None detected' })"
    
    if ($DryRun) {
        Write-Warning "DRY RUN MODE - No changes will be made"
    }
    
    # Check tools
    Test-Tools
    
    # Show installation guide if not auto-installing
    if (-not $AutoInstall -and $Global:MissingCount -gt 0) {
        Write-Host ""
        if ($packageManagers.Count -eq 0) {
            Show-PackageManagerInstallation
        }
        Show-InstallationGuide
        Write-Host ""
        Write-Info "To automatically install missing tools, run:"
        Write-Info "  .\setup-dev-environment.ps1 -AutoInstall"
    }
    
    Write-Host ""
    Write-Info "For more information, see: DEVELOPMENT_SETUP.md"
}

# Run main function
Main
