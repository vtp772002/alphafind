#!/usr/bin/env bash
# ==============================================================================
# AlphaFind 🚀 — Institutional Automated Installer with Live Progress
# ==============================================================================
# Usage:
#   curl -# -fsSL https://raw.githubusercontent.com/vtp772002/alphafind/main/install.sh | bash
# ==============================================================================

set -e

# Terminal colors & styling
RED='\033[0;31m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
BOLD='\033[1m'
DIM='\033[2m'
NC='\033[0m' # No Color

# Progress Bar Renderer
render_progress() {
    local pct=$1
    local msg=$2
    local extra=$3
    local width=22

    # Clamp percentage between 0 and 100
    if [ "$pct" -gt 100 ]; then pct=100; fi
    if [ "$pct" -lt 0 ]; then pct=0; fi

    local filled=$((pct * width / 100))
    local empty=$((width - filled))
    local bar=""
    local spacer=""

    for ((i=0; i<filled; i++)); do bar+="█"; done
    for ((i=0; i<empty; i++)); do spacer+="░"; done

    printf "\r  ${CYAN}%-32s${NC} ${GREEN}[%s%s]${NC} ${BOLD}%3d%%${NC} ${DIM}%-22s${NC}" "$msg" "$bar" "$spacer" "$pct" "$extra"
}

# Banner Display
echo -e "${CYAN}${BOLD}"
cat << "EOF"
    ___    __    ____  __  _____    ___________   ______ 
   /   |  / /   / __ \/ / / /   |  / ____/  _/ | / / __ \
  / /| | / /   / /_/ / /_/ / /| | / /_   / //  |/ / / / /
 / ___ |/ /___/ ____/ __  / ___ |/ __/ _/ // /|  / /_/ / 
/_/  |_/_____/_/   /_/ /_/_/  |_/_/   /___/_/ |_/_____/  
EOF
echo -e "${NC}"
echo -e "${BOLD}${CYAN}   🚀 AlphaFind Quant Engine — Automated Installer${NC}"
echo -e "${DIM}   Institutional Quantitative Mining Platform for WorldQuant BRAIN${NC}"
echo -e "${CYAN}═════════════════════════════════════════════════════════════════════════════${NC}\n"

# Installation destination determination
if [ "$PWD" = "$HOME" ]; then
    INSTALL_DIR="${HOME}/alphafind"
else
    INSTALL_DIR="${PWD}/alphafind"
fi

BIN_DIR="${HOME}/.cargo/bin"

# ------------------------------------------------------------------------------
# STEP 1/5: Verify Environment & Toolchain
# ------------------------------------------------------------------------------
render_progress 10 "[1/5] Verifying toolchain" "Checking Git..."

if ! command -v git &> /dev/null; then
    echo -e "\n${RED}❌ Git is not installed. Please install Git before running installer.${NC}"
    exit 1
fi

render_progress 40 "[1/5] Verifying toolchain" "Checking Cargo..."

if ! command -v cargo &> /dev/null; then
    render_progress 60 "[1/5] Verifying toolchain" "Installing Rustup..."
    echo ""
    echo -e "  ${YELLOW}⚠️  Rust/Cargo not detected. Installing official Rust toolchain...${NC}"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y > /dev/null 2>&1
    if [ -f "${HOME}/.cargo/env" ]; then
        source "${HOME}/.cargo/env"
    fi
fi

RUST_VER=$(rustc --version 2>/dev/null | awk '{print $2}' || echo "detected")
render_progress 100 "[1/5] Verifying toolchain" "Rust $RUST_VER (OK)"
echo ""

# ------------------------------------------------------------------------------
# STEP 2/5: Download AlphaFind Repository with Live % Progress
# ------------------------------------------------------------------------------
render_progress 0 "[2/5] Downloading repository" "Connecting to GitHub..."

if [ -d "$INSTALL_DIR/.git" ]; then
    render_progress 20 "[2/5] Updating repository" "Fetching main branch..."
    cd "$INSTALL_DIR"
    git fetch origin main > /dev/null 2>&1
    render_progress 70 "[2/5] Updating repository" "Syncing working tree..."
    git reset --hard origin/main > /dev/null 2>&1
    render_progress 100 "[2/5] Updating repository" "Updated to latest"
    echo ""
else
    # Parse git progress output in real-time
    git clone --progress --depth 1 https://github.com/vtp772002/alphafind.git "$INSTALL_DIR" 2>&1 | while IFS= read -r -d $'\r' line; do
        if [[ "$line" =~ ([0-9]+)%\ \(([0-9]+/[0-9]+)\) ]]; then
            pct="${BASH_REMATCH[1]}"
            count="${BASH_REMATCH[2]}"
            render_progress "$pct" "[2/5] Downloading repository" "$count objects"
        fi
    done
    render_progress 100 "[2/5] Downloading repository" "Download complete!"
    echo ""
    cd "$INSTALL_DIR"
fi

# ------------------------------------------------------------------------------
# STEP 3/5: Compile Native Optimized Binary
# ------------------------------------------------------------------------------
render_progress 0 "[3/5] Compiling release engine" "Analyzing dependencies..."

# Temporary build log file
BUILD_LOG=$(mktemp /tmp/alphafind_build_XXXXXX.log)

# Run cargo build in background and track progress
cargo build --release > "$BUILD_LOG" 2>&1 &
BUILD_PID=$!

chars="/-\|"
idx=0
pct=10

while kill -0 "$BUILD_PID" 2>/dev/null; do
    char="${chars:$((idx % 4)):1}"
    
    # Estimate progress from compiled crates
    crates_done=$(grep -c "Compiling " "$BUILD_LOG" 2>/dev/null | tail -n 1)
    crates_done=${crates_done:-0}
    if [ "$crates_done" -gt 0 ] 2>/dev/null; then
        # Total crates in dependency graph ~140-165
        pct=$((15 + (crates_done * 80 / 160)))
        if [ "$pct" -gt 95 ]; then pct=95; fi
        render_progress "$pct" "[3/5] Compiling release engine" "[$char] $crates_done crates built"
    else
        render_progress 15 "[3/5] Compiling release engine" "[$char] In progress..."
    fi
    
    idx=$((idx + 1))
    sleep 0.3
done

wait "$BUILD_PID"
rm -f "$BUILD_LOG"

render_progress 100 "[3/5] Compiling release engine" "Optimized build ready"
echo ""

# ------------------------------------------------------------------------------
# STEP 4/5: Install Native Binary to System PATH
# ------------------------------------------------------------------------------
render_progress 20 "[4/5] Installing binary to PATH" "Preparing directory..."
mkdir -p "$BIN_DIR"

render_progress 50 "[4/5] Installing binary to PATH" "Copying binary..."
cp target/release/alphafind "$BIN_DIR/alphafind"
chmod +x "$BIN_DIR/alphafind"

render_progress 80 "[4/5] Installing binary to PATH" "Configuring shell RC..."
SHELL_RC=""
if [ -n "$ZSH_VERSION" ] || [ "$SHELL" = "/bin/zsh" ]; then
    SHELL_RC="${HOME}/.zshrc"
elif [ -n "$BASH_VERSION" ] || [ "$SHELL" = "/bin/bash" ]; then
    SHELL_RC="${HOME}/.bashrc"
fi

if [ -n "$SHELL_RC" ] && [ -f "$SHELL_RC" ]; then
    if ! grep -q '\.cargo/bin' "$SHELL_RC"; then
        echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> "$SHELL_RC"
    fi
fi

render_progress 100 "[4/5] Installing binary to PATH" "~/.cargo/bin/alphafind"
echo ""

# ------------------------------------------------------------------------------
# STEP 5/5: Configure Environment & Secrets
# ------------------------------------------------------------------------------
render_progress 30 "[5/5] Configuring environment" "Checking .env template..."

if [ ! -f "$INSTALL_DIR/.env" ]; then
    if [ -f "$INSTALL_DIR/.env.example" ]; then
        cp "$INSTALL_DIR/.env.example" "$INSTALL_DIR/.env"
        render_progress 100 "[5/5] Configuring environment" ".env initialized"
    else
        render_progress 100 "[5/5] Configuring environment" "Ready"
    fi
else
    render_progress 100 "[5/5] Configuring environment" "Preserved existing .env"
fi
echo ""

# ------------------------------------------------------------------------------
# Installation Summary & Quickstart Guide
# ------------------------------------------------------------------------------
echo -e "\n${GREEN}═════════════════════════════════════════════════════════════════════════════${NC}"
echo -e "${BOLD}${GREEN}  ✅ AlphaFind Installation Completed Successfully! (100%)${NC}"
echo -e "${GREEN}═════════════════════════════════════════════════════════════════════════════${NC}"

echo -e "\n${BOLD}📁 Installed Directory:${NC}  ${CYAN}${INSTALL_DIR}${NC}"
echo -e "${BOLD}⚡ Binary Location:${NC}      ${CYAN}${BIN_DIR}/alphafind${NC}"

echo -e "\n${BOLD}🚀 Quickstart Guide:${NC}"
echo -e "  1. Switch to the installation directory:"
echo -e "     ${CYAN}cd ${INSTALL_DIR}${NC}"
echo -e "\n  2. Configure your WorldQuant BRAIN credentials in ${YELLOW}.env${NC}:"
echo -e "     ${YELLOW}nano .env${NC}"
echo -e "\n  3. Test connection across your accounts:"
echo -e "     ${CYAN}alphafind auth${NC}"
echo -e "\n  4. Synchronize your active Out-of-Sample portfolio:"
echo -e "     ${CYAN}alphafind sync${NC}"
echo -e "\n  5. Launch distributed 9-worker alpha mining:"
echo -e "     ${CYAN}alphafind screen --pillar OPTIONS --universe TOP1000${NC}\n"
