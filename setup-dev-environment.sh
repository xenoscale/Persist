#!/bin/bash

# Persist Development Environment Setup Script
# Cross-platform tool validation and installation for Unix-like systems
# Supports: macOS (Intel & Apple Silicon), Linux (Ubuntu, Debian, RHEL, CentOS, Fedora, Arch)
# Compatible with bash 3.x (macOS default) and bash 4.x+ (Linux default)
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

# Tool lists (bash 3.x compatible - using regular arrays)
TOOL_NAMES=(
    "rustc" "cargo" "rustfmt" "clippy"
    "python3" "pip" "maturin" "black" "ruff" "mypy" "pytest"
    "git" "make" "cmake"
)

TOOL_TYPES=(
    "required" "required" "recommended" "recommended"
    "required" "required" "recommended" "recommended" "recommended" "optional" "recommended"
    "required" "recommended" "optional"
)

TOOL_DESCRIPTIONS=(
    "Rust compiler"
    "Rust package manager"
    "Rust code formatter"
    "Rust linter"
    "Python interpreter (3.8+)"
    "Python package installer"
    "Python extension builder"
    "Python code formatter"
    "Python linter"
    "Python type checker"
    "Python testing framework"
    "Version control system"
    "Build automation tool"
    "Cross-platform build system"
)

TOOL_INSTALL_COMMANDS=(
    "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    "rustup component add rustfmt"
    "rustup component add clippy"
    "[System package manager] or python.org"
    "python3 -m ensurepip --upgrade"
    "pip install maturin"
    "pip install black"
    "pip install ruff"
    "pip install mypy"
    "pip install pytest pytest-cov"
    "[System package manager]"
    "[System package manager]"
    "[System package manager]"
)

# Helper functions for array lookups (bash 3.x compatible)
get_tool_index() {
    local tool_name="$1"
    local i=0
    for tool in "${TOOL_NAMES[@]}"; do
        if [ "$tool" = "$tool_name" ]; then
            echo $i
            return 0
        fi
        i=$((i + 1))
    done
    echo -1
}

get_tool_type() {
    local tool_name="$1"
    local index=$(get_tool_index "$tool_name")
    if [ "$index" -ge 0 ]; then
        echo "${TOOL_TYPES[$index]}"
    else
        echo "unknown"
    fi
}

get_tool_description() {
    local tool_name="$1"
    local index=$(get_tool_index "$tool_name")
    if [ "$index" -ge 0 ]; then
        echo "${TOOL_DESCRIPTIONS[$index]}"
    else
        echo "Unknown tool"
    fi
}

get_tool_install_command() {
    local tool_name="$1"
    local index=$(get_tool_index "$tool_name")
    if [ "$index" -ge 0 ]; then
        echo "${TOOL_INSTALL_COMMANDS[$index]}"
    else
        echo "Unknown installation method"
    fi
}

# Initialize tool definitions (now just a placeholder for compatibility)
init_tools() {
    # Arrays are already initialized above
    return 0
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
    
    local description=$(get_tool_description "$tool")
    local install_cmd=$(get_tool_install_command "$tool")
    
    print_info "Installing $tool ($description)..."
    
    if [ "$DRY_RUN" = true ]; then
        print_info "[DRY RUN] Would install $tool using: $install_cmd"
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
    
    for tool in "${TOOL_NAMES[@]}"; do
        local status=$(get_tool_type "$tool")
        local description=$(get_tool_description "$tool")
        local version=$(get_version "$tool")
        
        if command_exists "$tool"; then
            # Special check for Python version
            if [ "$tool" = "python3" ] && ! check_python_version; then
                missing_required+=("$tool")
                print_error "✗ $tool ($version) - $description - VERSION TOO OLD"
            else
                installed_tools+=("$tool")
                print_success "✓ $tool ($version) - $description"
                INSTALLED_COUNT=$((INSTALLED_COUNT + 1))
            fi
        else
            case $status in
                "required")
                    missing_required+=("$tool")
                    print_error "✗ $tool - $description [REQUIRED]"
                    ;;
                "recommended")
                    missing_recommended+=("$tool")
                    print_warning "✗ $tool - $description [RECOMMENDED]"
                    ;;
                "optional")
                    missing_optional+=("$tool")
                    print_info "✗ $tool - $description [OPTIONAL]"
                    ;;
            esac
            MISSING_COUNT=$((MISSING_COUNT + 1))
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
            local description=$(get_tool_description "$tool")
            local install_cmd=$(get_tool_install_command "$tool")
            echo "  ❌ $tool - $description"
            echo "     Install: $install_cmd"
        done
    fi
    
    if [ ${#missing_recommended[@]} -gt 0 ]; then
        echo
        print_warning "⚠️  Missing Recommended Tools (${#missing_recommended[@]}):"
        for tool in "${missing_recommended[@]}"; do
            local description=$(get_tool_description "$tool")
            local install_cmd=$(get_tool_install_command "$tool")
            echo "  ⚠️  $tool - $description"
            echo "     Install: $install_cmd"
        done
    fi
    
    if [ ${#missing_optional[@]} -gt 0 ]; then
        echo
        print_info "ℹ️  Missing Optional Tools (${#missing_optional[@]}):"
        for tool in "${missing_optional[@]}"; do
            local description=$(get_tool_description "$tool")
            local install_cmd=$(get_tool_install_command "$tool")
            echo "  ℹ️  $tool - $description"
            echo "     Install: $install_cmd"
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
    
    for tool in "${TOOL_NAMES[@]}"; do
        local status=$(get_tool_type "$tool")
        if [ "$status" = "required" ] || [ "$status" = "recommended" ]; then
            total_count=$((total_count + 1))
            if command_exists "$tool"; then
                if [ "$tool" = "python3" ] && ! check_python_version; then
                    continue
                fi
                success_count=$((success_count + 1))
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
