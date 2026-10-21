#!/usr/bin/env bash
# =============================================================================
# BOS Enterprise Distribution Packaging Pipeline (Linux / macOS / CI)
# =============================================================================

set -e

VERSION="${1:-0.1.0}"
OUTPUT_DIR="${2:-dist/bos-server}"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIST_PATH="$ROOT_DIR/$OUTPUT_DIR"
TAR_PATH="${DIST_PATH}-v${VERSION}-linux-x64.tar.gz"

echo "=============================================================="
echo " 🚀 BOS ENTERPRISE DISTRIBUTION BUILD PIPELINE (LINUX / UNIX)"
echo " Target Version : $VERSION"
echo " Root Directory : $ROOT_DIR"
echo " Output Target  : $DIST_PATH"
echo "=============================================================="

# 1. Verification of Required Tooling
echo -e "\n[1/5] Verifying toolchain environment..."
command -v cargo >/dev/null 2>&1 || { echo "Error: cargo (Rust toolchain) is required but not installed." >&2; exit 1; }
command -v node >/dev/null 2>&1 || { echo "Error: node is required to build the Svelte 5 frontend." >&2; exit 1; }
command -v npm >/dev/null 2>&1 || { echo "Error: npm is required." >&2; exit 1; }

echo "  ✓ Rust: $(cargo --version)"
echo "  ✓ Node: $(node --version)"

# 2. Compile Svelte 5 Enterprise Frontend
echo -e "\n[2/5] Building Svelte 5 Enterprise UI Studio..."
cd "$ROOT_DIR/ui"
if [ ! -d "node_modules" ]; then
    npm install --silent
fi
npm run build
cd "$ROOT_DIR"
echo "  ✓ Frontend compiled into ui/dist/"

# 3. Compile Production Rust Binaries
echo -e "\n[3/5] Building release binary for bosc-cli..."
cargo build --release -p bosc-cli
echo "  ✓ Release binary built successfully"

# 4. Assemble Distribution Layout
echo -e "\n[4/5] Assembling distribution package layout at $DIST_PATH..."
rm -rf "$DIST_PATH"
mkdir -p "$DIST_PATH/bin"
mkdir -p "$DIST_PATH/config"
mkdir -p "$DIST_PATH/ui/dist"
mkdir -p "$DIST_PATH/scripts"

# Copy binary
if [ -f "$ROOT_DIR/target/release/bosc" ]; then
    cp "$ROOT_DIR/target/release/bosc" "$DIST_PATH/bin/bosc"
elif [ -f "$ROOT_DIR/target/release/bosc.exe" ]; then
    cp "$ROOT_DIR/target/release/bosc.exe" "$DIST_PATH/bin/bosc.exe"
fi
chmod +x "$DIST_PATH/bin/bosc" 2>/dev/null || true

# Copy UI
cp -r "$ROOT_DIR/ui/dist/"* "$DIST_PATH/ui/dist/"

# Generate default node config
cat << 'EOF' > "$DIST_PATH/config/bos-node.toml"
# =============================================================================
# BOS Server Node Profile: Primary All-In-One Server
# =============================================================================

[node]
id = "bos-node-prod-01"
name = "BOS Enterprise Primary Node"
role = "all_in_one"

[network]
host = "0.0.0.0"
port = 3000
cors_origins = ["*"]

[ui]
enabled = true
static_dir = "./ui/dist"
index_file = "index.html"

[database]
url = "postgres://postgres:postgres@localhost:5432/bos_dev"
min_pool_size = 5
max_pool_size = 30
storage_strategy = "shared_rls"
connection_timeout_seconds = 10

[defaults]
default_tenant = "DEFAULT_TENANT"
default_company = "MAIN_COMPANY"

[nas_worker]
enabled = true
poll_interval_ms = 1000
batch_size = 50
process_outbox = true
process_job_queue = true
EOF

# Generate launcher
cat << 'EOF' > "$DIST_PATH/start.sh"
#!/usr/bin/env bash
cd "$(dirname "$0")"
echo "Starting BOS Enterprise Server Node..."
./bin/bosc node run --config config/bos-node.toml
EOF
chmod +x "$DIST_PATH/start.sh"

# 5. Compress Tarball Archive
echo -e "\n[5/5] Creating compressed distribution tarball..."
tar -czf "$TAR_PATH" -C "$(dirname "$DIST_PATH")" "$(basename "$DIST_PATH")"
if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$TAR_PATH" > "${TAR_PATH}.sha256"
elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$TAR_PATH" > "${TAR_PATH}.sha256"
fi

echo "=============================================================="
echo " 🎉 BOS DISTRIBUTION BUILD COMPLETE!"
echo " Standalone Folder : $DIST_PATH"
echo " Standalone Tarball: $TAR_PATH"
echo "=============================================================="
