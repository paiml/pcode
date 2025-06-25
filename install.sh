#!/bin/bash
# pcode installer script

set -e

REPO="paiml/pcode"
BINARY_NAME="pcode"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Print colored output
print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

# Detect OS and architecture
detect_platform() {
    OS="$(uname -s)"
    ARCH="$(uname -m)"
    
    case "$OS" in
        Linux)
            PLATFORM="linux"
            ;;
        Darwin)
            PLATFORM="darwin"
            ;;
        MINGW*|MSYS*|CYGWIN*)
            PLATFORM="windows"
            ;;
        *)
            print_error "Unsupported operating system: $OS"
            exit 1
            ;;
    esac
    
    case "$ARCH" in
        x86_64|amd64)
            ARCH="amd64"
            ;;
        arm64|aarch64)
            if [ "$PLATFORM" = "darwin" ]; then
                ARCH="arm64"
            else
                print_error "ARM64 Linux not yet supported"
                exit 1
            fi
            ;;
        *)
            print_error "Unsupported architecture: $ARCH"
            exit 1
            ;;
    esac
    
    if [ "$PLATFORM" = "windows" ]; then
        BINARY_NAME="${BINARY_NAME}.exe"
    fi
    
    ASSET_NAME="${BINARY_NAME}-${PLATFORM}-${ARCH}"
    if [ "$PLATFORM" = "windows" ]; then
        ASSET_NAME="${ASSET_NAME}.exe"
    fi
}

# Get latest release version
get_latest_version() {
    print_info "Fetching latest version..."
    VERSION=$(curl -s "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
    
    if [ -z "$VERSION" ]; then
        print_error "Failed to get latest version"
        exit 1
    fi
    
    print_info "Latest version: $VERSION"
}

# Download binary
download_binary() {
    URL="https://github.com/${REPO}/releases/download/${VERSION}/${ASSET_NAME}"
    print_info "Downloading from: $URL"
    
    if command -v wget > /dev/null; then
        wget -q --show-progress -O "$BINARY_NAME" "$URL"
    elif command -v curl > /dev/null; then
        curl -L --progress-bar -o "$BINARY_NAME" "$URL"
    else
        print_error "Neither wget nor curl found. Please install one of them."
        exit 1
    fi
    
    if [ ! -f "$BINARY_NAME" ]; then
        print_error "Download failed"
        exit 1
    fi
    
    chmod +x "$BINARY_NAME"
    print_success "Downloaded $BINARY_NAME"
}

# Install binary
install_binary() {
    # Check if running with sudo or as root
    if [ "$EUID" -eq 0 ] || [ -n "$SUDO_USER" ]; then
        INSTALL_DIR="/usr/local/bin"
    else
        INSTALL_DIR="$HOME/.local/bin"
        mkdir -p "$INSTALL_DIR"
        
        # Check if ~/.local/bin is in PATH
        if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
            print_warning "$INSTALL_DIR is not in your PATH"
            print_info "Add the following to your shell configuration file:"
            print_info "  export PATH=\"\$PATH:$INSTALL_DIR\""
        fi
    fi
    
    print_info "Installing to $INSTALL_DIR"
    
    if [ -f "$INSTALL_DIR/$BINARY_NAME" ]; then
        print_warning "Existing installation found at $INSTALL_DIR/$BINARY_NAME"
        read -p "Overwrite? (y/N) " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            print_info "Installation cancelled"
            exit 0
        fi
    fi
    
    if [ "$EUID" -eq 0 ] || [ -n "$SUDO_USER" ]; then
        mv "$BINARY_NAME" "$INSTALL_DIR/"
    else
        mv "$BINARY_NAME" "$INSTALL_DIR/" || {
            print_error "Failed to install. Try running with sudo."
            exit 1
        }
    fi
    
    print_success "Installed $BINARY_NAME to $INSTALL_DIR"
}

# Verify installation
verify_installation() {
    if command -v "$BINARY_NAME" > /dev/null; then
        print_success "Installation verified"
        print_info "Version: $($BINARY_NAME --version)"
    else
        print_warning "Binary installed but not found in PATH"
        print_info "You may need to restart your shell or add the install directory to PATH"
    fi
}

# Main installation flow
main() {
    echo "🤖 pcode Installer"
    echo "=================="
    echo
    
    detect_platform
    print_info "Detected platform: $PLATFORM-$ARCH"
    
    get_latest_version
    download_binary
    install_binary
    verify_installation
    
    echo
    print_success "Installation complete!"
    echo
    echo "Get started with:"
    echo "  $BINARY_NAME --help"
    echo
    echo "For interactive mode:"
    echo "  $BINARY_NAME"
    echo
}

# Run main function
main "$@"