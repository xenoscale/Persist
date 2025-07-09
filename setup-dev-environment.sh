#!/bin/bash

# Persist Development Environment Setup Script
# Cross-platform tool validation and installation for Unix-like systems
# Supports: macOS (Intel & Apple Silicon), Linux (Ubuntu, Debian, RHEL, CentOS, Fedora, Arch)
# Usage: ./setup-dev-environment.sh [OPTIONS]

set -e

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
PURPLE='\033[0;35m'
NC='\033[0m' # No Color

# Configuration
AUTO_INSTALL=false
VERBOSE=false
DRY_RUN=false

# Counters
INSTALLED_COUNT=0
MISSING_COUNT=0

# Tool lists
declare -A TOOLS
declare -A TOOL_DESCRIPTIONS
declare -A INSTALL_COMMANDS
declare -A VERSIONS

# Initialize tool definitions
init_tools() {
    # Core tools
    TOOLS["rustc"]="required"
    TOOL_DESCRIPTIONS["rustc"]="Rust compiler"
    INSTALL_COMMANDS["rustc"]="curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    
    TOOLS["cargo"]="required"
    TOOL_DESCRIPTIONS["cargo"]="Rust package manager"
    INSTALL_COMMANDS["cargo"]="curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    
    TOOLS["rustfmt"]="recommended"
    TOOL_DESCRIPTIONS["rustfmt"]="Rust code formatter"
    INSTALL_COMMANDS["rustfmt"]="rustup component add rustfmt"
    
    TOOLS["clippy"]="recommended"
    TOOL_DESCRIPTIONS["clippy"]="Rust linter"
    INSTALL_COMMANDS["clippy"]="rustup component add clippy"
    
    # Python tools
    TOOLS["python3"]="required"
    TOOL_DESCRIPTIONS["python3"]="Python interpreter (3.8+)"
    INSTALL_COMMANDS["python3"]="[System package manager] or python.org"
    
    TOOLS["pip"]="required"
    TOOL_DESCRIPTIONS["pip"]="Python package installer"
    INSTALL_COMMANDS["pip"]="python3 -m ensurepip --upgrade"
    
    TOOLS["maturin"]="recommended"
    TOOL_DESCRIPTIONS["maturin"]="Python extension builder"
    INSTALL_COMMANDS["maturin"]="pip install maturin"
    
    TOOLS["black"]="recommended"
    TOOL_DESCRIPTIONS["black"]="Python code formatter"
    INSTALL_COMMANDS["black"]="pip install black"
    
    TOOLS["ruff"]="recommended"
    TOOL_DESCRIPTIONS["ruff"]="Python linter"
    INSTALL_COMMANDS["ruff"]="pip install ruff"
    
    TOOLS["mypy"]="optional"
    TOOL_DESCRIPTIONS["mypy"]="Python type checker"
    INSTALL_COMMANDS["mypy"]="pip install mypy"
    
    TOOLS["pytest"]="recommended"
    TOOL_DESCRIPTIONS["pytest"]="Python testing framework"
    INSTALL_COMMANDS["pytest"]="pip install pytest pytest-cov"
    
    # Build tools
    TOOLS["git"]="required"
    TOOL_DESCRIPTIONS["git"]="Version control system"
    INSTALL_COMMANDS["git"]="[System package manager]"
    
    TOOLS["make"]="recommended"
    TOOL_DESCRIPTIONS["make"]="Build automation tool"
    INSTALL_COMMANDS["make"]="[System package manager]"
    
    TOOLS["cmake"]="optional"
    TOOL_DESCRIPTIONS["cmake"]="Cross-platform build system"
    INSTALL_COMMANDS["cmake"]="[System package manager]"
}

# Print functions
print_header() {
    echo -e "\n${PURPLE}====================================================${NC}"
    echo -e "${PURPLE}$1${NC}"
    echo -e "${PURPLE}====================================================${NC}\n"
}

print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_verbose() {
    if [ "$VERBOSE" = true ]; then
        echo -e "${CYAN}[VERBOSE]${NC} $1"
    fi
}

# Detect operating system and package manager
detect_os() {
    if [[ "$OSTYPE" == "darwin"* ]]; then
        OS="macos"
        if command -v brew >/dev/null 2>&1; then
            PKG_MANAGER="brew"
        else
            PKG_MANAGER="none"
        fi
    elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
        OS="linux"
        
        # Detect Linux distribution and package manager
        if command -v apt >/dev/null 2>&1; then
            PKG_MANAGER="apt"
            DISTRO="debian"
        elif command -v yum >/dev/null 2>&1; then
            PKG_MANAGER="yum"
            DISTRO="rhel"
        elif command -v dnf >/dev/null 2>&1; then
            PKG_MANAGER="dnf"
            DISTRO="fedora"
        elif command -v pacman >/dev/null 2>&1; then
            PKG_MANAGER="pacman"
            DISTRO="arch"
        elif command -v zypper >/dev/null 2>&1; then
            PKG_MANAGER="zypper"
            DISTRO="suse"
        else
            PKG_MANAGER="none"
            DISTRO="unknown"
        fi
    else
        OS="unknown"
        PKG_MANAGER="none"
    fi
    
    print_verbose "Detected OS: $OS"
    print_verbose "Package manager: $PKG_MANAGER"
    if [ -n "$DISTRO" ]; then
        print_verbose "Distribution: $DISTRO"
    fi
}

# Check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Get version of a tool
get_version() {
    local tool="$1"
    local version=""
    
    case $tool in
        "rustc")
            version=$(rustc --version 2>/dev/null | cut -d' ' -f2 || echo "unknown")
            ;;
        "cargo")
            version=$(cargo --version 2>/dev/null | cut -d' ' -f2 || echo "unknown")
            ;;
        "python3")
            version=$(python3 --version 2>/dev/null | cut -d' ' -f2 || echo "unknown")
            ;;
        "pip")
            version=$(pip --version 2>/dev/null | cut -d' ' -f2 || pip3 --version 2>/dev/null | cut -d' ' -f2 || echo "unknown")
            ;;
        "git")
            version=$(git --version 2>/dev/null | cut -d' ' -f3 || echo "unknown")
            ;;
        *)
            # Try generic --version
            if command_exists "$tool"; then
                version=$($tool --version 2>/dev/null | head -n1 | grep -oE '[0-9]+\.[0-9]+(\.[0-9]+)?' | head -n1 || echo "installed")
            else
                version="not found"
            fi
            ;;
    esac
    
    echo "$version"
}

# Check Python version compatibility
check_python_version() {
    if command_exists python3; then
        local version=$(python3 -c "import sys; print(f'{sys.version_info.major}.{sys.version_info.minor}')" 2>/dev/null)
        local major=$(echo "$version" | cut -d. -f1)
        local minor=$(echo "$version" | cut -d. -f2)
        
        if [ "$major" -ge 3 ] && [ "$minor" -ge 8 ]; then
            return 0
        else
            print_warning "Python $version found, but 3.8+ is required"
            return 1
        fi
    else
        return 1
    fi
}

# Install Homebrew on macOS
install_homebrew() {
    if [ "$OS" = "macos" ] && [ "$PKG_MANAGER" = "none" ]; then
        print_info "Installing Homebrew package manager..."
        if [ "$DRY_RUN" = true ]; then
            print_info "[DRY RUN] Would install Homebrew"
            return 0
        fi
        
        /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
        
        # Add to PATH for Apple Silicon Macs
        if [[ $(uname -m) == "arm64" ]]; then
            echo 'eval "$(/opt/homebrew/bin/brew shellenv)"' >> ~/.zprofile
            eval "$(/opt/homebrew/bin/brew shellenv)"
        else
            echo 'eval "$(/usr/local/bin/brew shellenv)"' >> ~/.zprofile
            eval "$(/usr/local/bin/brew shellenv)"
        fi
        
        PKG_MANAGER="brew"
        print_success "Homebrew installed successfully"
    fi
}

# Install a tool using the appropriate package manager
install_tool() {
    local tool="$1"
    local success=false
    
    print_info "Installing $tool (${TOOL_DESCRIPTIONS[$tool]})..."
    
    if [ "$DRY_RUN" = true ]; then
        print_info "[DRY RUN] Would install $tool using: ${INSTALL_COMMANDS[$tool]}"
        return 0
    fi
    
    case $tool in
        "rustc"|"cargo")
            if ! command_exists rustup; then
                print_info "Installing Rust toolchain via rustup..."
                curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
                source ~/.cargo/env
                success=true
            else
                print_info "Rust toolchain already available via rustup"
                success=true
            fi
            ;;
        "rustfmt"|"clippy")
            if command_exists rustup; then
                rustup component add "$tool"
                success=true
            else
                print_error "rustup not found. Install Rust first."
                success=false
            fi
            ;;
        "python3")
            install_python3
            success=$?
            ;;
        "pip")
            if command_exists python3; then
                python3 -m ensurepip --upgrade
                success=true
            else
                print_error "python3 not found. Install Python first."
                success=false
            fi
            ;;
        "maturin"|"black"|"ruff"|"mypy"|"pytest")
            install_python_package "$tool"
            success=$?
            ;;
        "git"|"make"|"cmake")
            install_system_tool "$tool"
            success=$?
            ;;
        *)
            print_warning "Don't know how to install $tool automatically"
            success=false
            ;;
    esac
    
    if [ $success -eq 0 ]; then
        print_success "$tool installed successfully"
    else
        print_error "Failed to install $tool"
    fi
    
    return $success
}

# Install Python 3
install_python3() {
    case $PKG_MANAGER in
        "brew")
            brew install python@3.11
            ;;
        "apt")
            sudo apt update && sudo apt install -y python3 python3-pip python3-dev
            ;;
        "yum")
            sudo yum install -y python3 python3-pip python3-devel
            ;;
        "dnf")
            sudo dnf install -y python3 python3-pip python3-devel
            ;;
        "pacman")
            sudo pacman -S python python-pip
            ;;
        "zypper")
            sudo zypper install -y python3 python3-pip python3-devel
            ;;
        *)
            print_error "Please install Python 3.8+ manually from https://python.org"
            return 1
            ;;
    esac
}

# Install Python package
install_python_package() {
    local package="$1"
    
    if ! command_exists pip && ! command_exists pip3; then
        print_error "pip not found. Install pip first."
        return 1
    fi
    
    # Try pip3 first, then pip
    if command_exists pip3; then
        pip3 install "$package"
    else
        pip install "$package"
    fi
}

# Install system tool
install_system_tool() {
    local tool="$1"
    
    case $PKG_MANAGER in
        "brew")
            brew install "$tool"
            ;;
        "apt")
            sudo apt update && sudo apt install -y "$tool"
            ;;
        "yum")
            sudo yum install -y "$tool"
            ;;
        "dnf")
            sudo dnf install -y "$tool"
            ;;
        "pacman")
            sudo pacman -S "$tool"
            ;;
        "zypper")
            sudo zypper install -y "$tool"
            ;;
        *)
            print_error "Please install $tool manually using your system package manager"
            return 1
            ;;
    esac
}

# Check all tools
check_tools() {
    print_header "Checking Development Tools"
    
    local installed_tools=()
    local missing_required=()
    local missing_recommended=()
    local missing_optional=()
    
    for tool in "${!TOOLS[@]}"; do
        local status="${TOOLS[$tool]}"
        local version=$(get_version "$tool")
        
        if command_exists "$tool"; then
            # Special check for Python version
            if [ "$tool" = "python3" ] && ! check_python_version; then
                missing_required+=("$tool")
                print_error "✗ $tool ($version) - ${TOOL_DESCRIPTIONS[$tool]} - VERSION TOO OLD"
            else
                installed_tools+=("$tool")
                print_success "✓ $tool ($version) - ${TOOL_DESCRIPTIONS[$tool]}"
                ((INSTALLED_COUNT++))
            fi
        else
            case $status in
                "required")
                    missing_required+=("$tool")
                    print_error "✗ $tool - ${TOOL_DESCRIPTIONS[$tool]} [REQUIRED]"
                    ;;
                "recommended")
                    missing_recommended+=("$tool")
                    print_warning "✗ $tool - ${TOOL_DESCRIPTIONS[$tool]} [RECOMMENDED]"
                    ;;
                "optional")
                    missing_optional+=("$tool")
                    print_info "✗ $tool - ${TOOL_DESCRIPTIONS[$tool]} [OPTIONAL]"
                    ;;
            esac
            ((MISSING_COUNT++))
        fi
    done
    
    # Summary
    echo
    if [ ${#installed_tools[@]} -gt 0 ]; then
        print_success "✅ Installed Tools (${#installed_tools[@]}):"
        for tool in "${installed_tools[@]}"; do
            echo "  ✅ $tool"
        done
    fi
    
    if [ ${#missing_required[@]} -gt 0 ]; then
        echo
        print_error "❌ Missing Required Tools (${#missing_required[@]}):"
        for tool in "${missing_required[@]}"; do
            echo "  ❌ $tool - ${TOOL_DESCRIPTIONS[$tool]}"
            echo "     Install: ${INSTALL_COMMANDS[$tool]}"
        done
    fi
    
    if [ ${#missing_recommended[@]} -gt 0 ]; then
        echo
        print_warning "⚠️  Missing Recommended Tools (${#missing_recommended[@]}):"
        for tool in "${missing_recommended[@]}"; do
            echo "  ⚠️  $tool - ${TOOL_DESCRIPTIONS[$tool]}"
            echo "     Install: ${INSTALL_COMMANDS[$tool]}"
        done
    fi
    
    if [ ${#missing_optional[@]} -gt 0 ]; then
        echo
        print_info "ℹ️  Missing Optional Tools (${#missing_optional[@]}):"
        for tool in "${missing_optional[@]}"; do
            echo "  ℹ️  $tool - ${TOOL_DESCRIPTIONS[$tool]}"
            echo "     Install: ${INSTALL_COMMANDS[$tool]}"
        done
    fi
    
    # Auto-install if requested
    if [ "$AUTO_INSTALL" = true ]; then
        echo
        local all_missing=("${missing_required[@]}" "${missing_recommended[@]}")
        
        if [ ${#all_missing[@]} -gt 0 ]; then
            print_header "Auto-Installing Missing Tools"
            
            # Install Homebrew on macOS if needed
            install_homebrew
            
            for tool in "${all_missing[@]}"; do
                install_tool "$tool"
            done
            
            print_header "Re-checking Tools After Installation"
            check_tools_final
        else
            print_success "All required and recommended tools are already installed!"
        fi
    fi
}

# Final check after installation (simpler version)
check_tools_final() {
    local success_count=0
    local total_count=0
    
    for tool in "${!TOOLS[@]}"; do
        local status="${TOOLS[$tool]}"
        if [ "$status" = "required" ] || [ "$status" = "recommended" ]; then
            ((total_count++))
            if command_exists "$tool"; then
                if [ "$tool" = "python3" ] && ! check_python_version; then
                    continue
                fi
                ((success_count++))
                print_success "✓ $tool"
            else
                print_error "✗ $tool"
            fi
        fi
    done
    
    echo
    if [ $success_count -eq $total_count ]; then
        print_success "🎉 All required and recommended tools are now installed!"
        print_info "You can now run: ./build-and-test.sh"
    else
        print_warning "Some tools are still missing ($success_count/$total_count installed)"
        print_info "You may need to restart your terminal or source ~/.cargo/env for Rust tools"
    fi
}

# Show installation guide
show_installation_guide() {
    print_header "Manual Installation Guide"
    
    case $OS in
        "macos")
            echo "macOS Installation Commands:"
            echo
            echo "1. Install Homebrew (if not installed):"
            echo "   /bin/bash -c \"\$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\""
            echo
            echo "2. Install development tools:"
            echo "   brew install git python@3.11 cmake"
            echo
            echo "3. Install Rust:"
            echo "   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
            echo "   source ~/.cargo/env"
            echo "   rustup component add rustfmt clippy"
            echo
            echo "4. Install Python development tools:"
            echo "   pip3 install maturin black ruff mypy pytest pytest-cov"
            ;;
        "linux")
            echo "Linux Installation Commands:"
            echo
            case $PKG_MANAGER in
                "apt")
                    echo "Ubuntu/Debian:"
                    echo "   sudo apt update"
                    echo "   sudo apt install -y git python3 python3-pip python3-dev build-essential cmake"
                    ;;
                "yum")
                    echo "RHEL/CentOS 7:"
                    echo "   sudo yum groupinstall -y \"Development Tools\""
                    echo "   sudo yum install -y git python3 python3-pip python3-devel cmake"
                    ;;
                "dnf")
                    echo "Fedora/RHEL 8+:"
                    echo "   sudo dnf groupinstall -y \"Development Tools\""
                    echo "   sudo dnf install -y git python3 python3-pip python3-devel cmake"
                    ;;
                "pacman")
                    echo "Arch Linux:"
                    echo "   sudo pacman -S base-devel git python python-pip cmake"
                    ;;
                *)
                    echo "For your Linux distribution, install:"
                    echo "   - git, python3, python3-pip, python3-dev"
                    echo "   - build-essential or development tools"
                    echo "   - cmake (optional)"
                    ;;
            esac
            echo
            echo "Install Rust (all distributions):"
            echo "   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
            echo "   source ~/.cargo/env"
            echo "   rustup component add rustfmt clippy"
            echo
            echo "Install Python development tools:"
            echo "   pip3 install maturin black ruff mypy pytest pytest-cov"
            ;;
        *)
            echo "For your system, please install manually:"
            echo "- Rust toolchain: https://rustup.rs/"
            echo "- Python 3.8+: https://python.org"
            echo "- Git and build tools via your system package manager"
            ;;
    esac
}

# Show usage information
show_usage() {
    cat << EOF
Persist Development Environment Setup Script

USAGE:
    ./setup-dev-environment.sh [OPTIONS]

DESCRIPTION:
    This script checks for required development tools and optionally installs them.
    It supports macOS, Linux, and Windows (WSL/Git Bash) environments.

OPTIONS:
    --auto-install    Automatically install missing tools
    --verbose         Enable verbose output
    --dry-run         Show what would be done without executing
    --help            Show this help message

EXAMPLES:
    # Check what tools are installed/missing
    ./setup-dev-environment.sh

    # Automatically install missing tools
    ./setup-dev-environment.sh --auto-install

    # See what would be installed without making changes
    ./setup-dev-environment.sh --auto-install --dry-run

    # Get detailed output during installation
    ./setup-dev-environment.sh --auto-install --verbose

EOF
}

# Main function
main() {
    # Parse command line arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --auto-install)
                AUTO_INSTALL=true
                shift
                ;;
            --verbose)
                VERBOSE=true
                shift
                ;;
            --dry-run)
                DRY_RUN=true
                shift
                ;;
            --help|-h)
                show_usage
                exit 0
                ;;
            *)
                print_error "Unknown option: $1"
                show_usage
                exit 1
                ;;
        esac
    done
    
    print_header "Persist Development Environment Setup"
    
    # Initialize
    init_tools
    detect_os
    
    print_info "Platform: $OS"
    print_info "Package Manager: $PKG_MANAGER"
    
    if [ "$DRY_RUN" = true ]; then
        print_warning "DRY RUN MODE - No changes will be made"
    fi
    
    # Check tools
    check_tools
    
    # Show installation guide if not auto-installing
    if [ "$AUTO_INSTALL" = false ] && [ $MISSING_COUNT -gt 0 ]; then
        echo
        show_installation_guide
        echo
        print_info "To automatically install missing tools, run:"
        print_info "  ./setup-dev-environment.sh --auto-install"
    fi
    
    echo
    print_info "For more information, see: DEVELOPMENT_SETUP.md"
}

# Run main function with all arguments
main "$@"
