# Development Environment Setup Guide

This guide covers setting up your development environment for the Persist project across different platforms.

## Quick Start

The easiest way to set up your development environment is using our automated setup scripts:

```bash
# Automatic installation (recommended)
./setup-dev-tools --auto-install

# Manual guidance (shows what needs to be installed)
./setup-dev-tools

# Verbose output with detailed information
./setup-dev-tools --verbose --auto-install
```

## Platform-Specific Setup Scripts

### Unix-like Systems (Linux, macOS, WSL)

Use the bash script for comprehensive setup:

```bash
# Automatic installation
./setup-dev-environment.sh --auto-install

# See what would be installed without making changes
./setup-dev-environment.sh --dry-run

# Get detailed output
./setup-dev-environment.sh --verbose
```

### Windows PowerShell

For native Windows PowerShell environments:

```powershell
# Automatic installation
.\setup-dev-environment.ps1 -AutoInstall

# See what would be installed without making changes
.\setup-dev-environment.ps1 -DryRun

# Get detailed output
.\setup-dev-environment.ps1 -Verbose
```

## Required Tools

### Core Requirements

| Tool | Purpose | Installation |
|------|---------|--------------| 
| **Rust Toolchain** | Core language and build system | Via [rustup](https://rustup.rs/) |
| **Python 3.8+** | Python bindings and testing | System package manager or [python.org](https://python.org) |
| **Git** | Version control | System package manager |

### Development Tools

| Tool | Purpose | Installation |
|------|---------|--------------| 
| **maturin** | Python extension building | `pip install maturin` |
| **rustfmt** | Rust code formatting | `rustup component add rustfmt` |
| **clippy** | Rust linting | `rustup component add clippy` |
| **black** | Python code formatting | `pip install black` |
| **ruff** | Python linting | `pip install ruff` |
| **pytest** | Python testing | `pip install pytest` |
| **mypy** | Python type checking | `pip install mypy` |

### Build Dependencies

| Tool | Purpose | Platform |
|------|---------|----------|
| **make** | Build automation | Linux/macOS: system packages, Windows: MinGW/MSYS2 |
| **cmake** | Cross-platform build | All platforms via package managers |
| **C++ Compiler** | Native compilation | GCC/Clang (Linux/macOS), MSVC/MinGW (Windows) |

## Platform-Specific Details

### macOS Setup

The script supports both Intel and Apple Silicon Macs:

```bash
# Install Homebrew if not present
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Run setup script
./setup-dev-tools --auto-install
```

**Package Manager Support:**
- **Homebrew** (recommended): `brew install git make cmake`
- **MacPorts**: Manual installation required
- **Xcode Command Line Tools**: Automatically prompted if needed

### Linux Setup

Supports major distributions with automatic package manager detection:

**Ubuntu/Debian:**
```bash
sudo apt update
./setup-dev-tools --auto-install
```

**RHEL/CentOS/Fedora:**
```bash
# For CentOS/RHEL 7
sudo yum groupinstall "Development Tools"

# For Fedora/RHEL 8+
sudo dnf groupinstall "Development Tools"

./setup-dev-tools --auto-install
```

**Arch Linux:**
```bash
sudo pacman -S base-devel
./setup-dev-tools --auto-install
```

### Windows Setup

Multiple setup options depending on your development environment:

#### Option 1: WSL2 (Recommended)
```bash
# In WSL2 terminal
./setup-dev-tools --auto-install
```

#### Option 2: Git Bash/MSYS2
```bash
# In Git Bash
./setup-dev-tools --auto-install
```

#### Option 3: Native PowerShell
```powershell
# In PowerShell (as Administrator if using Chocolatey)
.\setup-dev-environment.ps1 -AutoInstall
```

**Package Manager Support:**
- **winget**: `winget install Git.Git Kitware.CMake Microsoft.VisualStudio.2022.BuildTools`
- **Chocolatey**: `choco install git cmake visualstudio2022buildtools`
- **Scoop**: `scoop install git cmake mingw`

## Manual Installation Guide

If you prefer to install tools manually or the automated scripts don't work for your system:

### 1. Install Rust

```bash
# Install rustup (Rust installer)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Source the environment
source ~/.cargo/env

# Install required components
rustup component add rustfmt clippy
```

### 2. Install Python Development Tools

```bash
# Ensure you have Python 3.8+ and pip
python3 --version
pip3 --version

# Install Python development tools
pip3 install maturin black ruff mypy pytest pytest-cov
```

### 3. Install Build Tools

**macOS:**
```bash
# Install Xcode Command Line Tools
xcode-select --install

# Install Homebrew
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Install additional tools
brew install git make cmake
```

**Linux (Ubuntu/Debian):**
```bash
sudo apt update
sudo apt install -y git build-essential cmake python3-dev
```

**Linux (RHEL/CentOS/Fedora):**
```bash
# RHEL/CentOS 7
sudo yum groupinstall -y "Development Tools"
sudo yum install -y git cmake python3-devel

# Fedora/RHEL 8+
sudo dnf groupinstall -y "Development Tools"
sudo dnf install -y git cmake python3-devel
```

**Windows:**
1. Install [Git for Windows](https://git-scm.com/download/win)
2. Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/) or [Visual Studio Community](https://visualstudio.microsoft.com/vs/community/)
3. Install [CMake](https://cmake.org/download/)
4. Install Python from [python.org](https://python.org)

## Validation

After setup, validate your environment:

```bash
# Run validation script
./setup-dev-tools

# Or test with the build script
./build-and-test.sh --quick
```

Expected output should show all tools as installed:
```
✅ Installed Tools (12):
  ✅ rustc
  ✅ cargo
  ✅ rustfmt
  ✅ clippy
  ✅ python
  ✅ pip
  ✅ maturin
  ✅ black
  ✅ ruff
  ✅ mypy
  ✅ pytest
  ✅ git
```

## Troubleshooting

### Common Issues

#### "Command not found" errors
- **Cause**: Tool not in PATH or not installed
- **Solution**: Run `./setup-dev-tools` to check what's missing

#### Python version too old
- **Cause**: System Python is older than 3.8
- **Solution**: Install newer Python via package manager or pyenv

#### Permission denied on Windows
- **Cause**: PowerShell execution policy restrictions
- **Solution**: Run `Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser`

#### Rust tools not found after installation
- **Cause**: Shell environment not updated
- **Solution**: Restart terminal or run `source ~/.cargo/env`

### Platform-Specific Issues

#### macOS
- **Xcode Command Line Tools**: Required for native compilation
- **Homebrew permissions**: May need `sudo` for system-wide installation
- **Apple Silicon**: Ensure you're using native tools, not Rosetta

#### Linux
- **Missing development headers**: Install `python3-dev` or `python3-devel`
- **Permission issues**: Use package manager instead of global pip install

#### Windows
- **Long path support**: Enable in Windows settings for deep directory structures
- **Antivirus interference**: May need to whitelist Rust/cargo directories
- **WSL vs native**: Choose one environment and stick with it for consistency

## Environment Variables

The setup scripts may set these environment variables:

| Variable | Purpose | Default |
|----------|---------|---------| 
| `CARGO_HOME` | Cargo installation directory | `~/.cargo` |
| `RUSTUP_HOME` | Rustup installation directory | `~/.rustup` |
| `PATH` | Include Rust and Python tools | Updated automatically |

## IDE Setup

### VS Code (Recommended)

Install the following extensions:
- `rust-lang.rust-analyzer` - Rust language support
- `ms-python.python` - Python language support
- `ms-python.black-formatter` - Python formatting
- `charliermarsh.ruff` - Python linting

### Other IDEs

- **CLion**: Native Rust support with IntelliJ Rust plugin
- **Vim/Neovim**: Use `rust-analyzer` LSP and Python LSP
- **Emacs**: `rustic-mode` and `python-mode`

## Continuous Integration

The setup scripts are designed to work in CI environments:

```yaml
# GitHub Actions example
- name: Setup development environment
  run: ./setup-dev-tools --auto-install --verbose
```

## Integration with Build Scripts

The setup scripts are integrated with the main build script:

```bash
# The build script will now suggest setup commands when tools are missing
./build-and-test.sh
```

If tools are missing, you'll see output like:
```
[ERROR] Missing required tools:
  - maturin (Python extension builder)

[INFO] Run the setup script to install missing tools:
  ./setup-dev-tools --auto-install
```

## Next Steps

After setting up your development environment:

1. **Build the project**: `./build-and-test.sh`
2. **Run tests**: `./build-and-test.sh --quick`
3. **Format code**: `cargo fmt --all && black persist-python/`
4. **Lint code**: `cargo clippy --all-targets -- -D warnings`

For more information, see:
- [CONTRIBUTING.md](CONTRIBUTING.md) - Contribution guidelines
- [DEVELOPMENT.md](DEVELOPMENT.md) - Development workflow
- [README.md](README.md) - Project overview
