# BOS Distribution & Packaging Guide

> **Document ID:** `BOS-DOC-14`  
> **Source Verification:** `scripts/build-distribution.ps1`, `scripts/build-distribution.sh`, `crates/bosc-cli`  
> **Target Audience:** DevOps Engineers, System Administrators, Cloud Architects

---

## 1. Overview & Packaging Philosophy

The BOS Distribution Pipeline packages the entire 3-tier enterprise stack—the Rust systems runtime (`bosc`), the compiled Svelte 5 UI Studio, configuration profiles, and 1-click execution scripts—into a **zero-dependency, standalone distribution bundle**.

Target deployment systems **do not require**:
* Rust / `cargo`
* Node.js / `npm`
* Visual Studio / C++ Build Tools
* External web servers (Nginx/IIS are optional, not mandatory)

The output is a self-contained archive (`.zip` on Windows, `.tar.gz` on Linux) containing the compiled binary, minified UI single-page application, and declarative server node profiles.

---

## 2. Distribution Package Structure

When unpacked, the distribution bundle conforms to the following directory layout:

```
bos-server/
├── bin/
│   └── bosc.exe                # Stripped, production-optimized release binary
├── ui/
│   └── dist/                   # Minified Svelte 5 Single Page Application
│       ├── index.html
│       ├── favicon.svg
│       └── assets/             # Bundled JS, CSS, Monaco Editor, Lucide icons
├── config/
│   ├── bos-node.toml           # Primary All-In-One server profile (UI + API + NAS)
│   └── bos-nas.toml            # Dedicated headless NAS background batch worker profile
├── start.bat                   # 1-click Windows batch launcher
├── start.ps1                   # 1-click PowerShell launcher
├── start.sh                    # 1-click Linux/macOS bash launcher
└── QUICKSTART.md               # Simple, non-IT user quickstart manual
```

---

## 3. Automated Packaging Scripts

BOS provides dual automated distribution build pipelines:

### 3.1. Windows / PowerShell (`scripts/build-distribution.ps1`)

```powershell
# Full end-to-end compilation & packaging:
pwsh ./scripts/build-distribution.ps1

# Fast repackage (using already built UI and binaries):
pwsh ./scripts/build-distribution.ps1 -SkipFrontend -SkipBackend

# Custom output directory or version:
pwsh ./scripts/build-distribution.ps1 -Version "1.0.0" -OutputDir "dist/custom-server"
```

#### Pipeline Steps:
1. **Toolchain Verification**: Validates `cargo`, `node`, and `npm`.
2. **Frontend Compilation**: Builds Svelte 5 studio (`npm run build` in `ui/`).
3. **Backend Compilation**: Compiles release binary (`cargo build --release -p bosc-cli`).
4. **Assembly**: Layouts `bin/`, `ui/dist/`, `config/`, and launcher scripts.
5. **Configuration Profiles**: Emits `bos-node.toml` and `bos-nas.toml`.
6. **Archive & Checksum**: Packages into `bos-server-v{VERSION}-windows-x64.zip` and generates a companion `.sha256` checksum file.

---

### 3.2. Linux / macOS Bash (`scripts/build-distribution.sh`)

```bash
# Full build on Linux / CI/CD:
./scripts/build-distribution.sh "0.1.0" "dist/bos-server"
```

---

## 4. End-User Quickstart on Target Machines

Once unpacked on an enterprise server or user workstation:

1. **Configure Connection (`config/bos-node.toml`)**:
   ```toml
   [database]
   url = "postgres://postgres:Secret123@db-host:5432/bos_prod"
   ```

2. **Validate Setup**:
   ```bash
   ./bin/bosc node check --config config/bos-node.toml
   ```

3. **Launch Server**:
   Double click `start.bat` (or run `./start.sh`). The server immediately serves both the Web UI and API at **`http://localhost:3000`**.
