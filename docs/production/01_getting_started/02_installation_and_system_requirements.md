# Volume 1: Getting Started
## 02. Installation & System Requirements

This document provides complete system specifications, prerequisite toolchain setup, and environment verification instructions for running BOS in development and enterprise production environments.

---

## 🖥️ System Requirements

### Hardware Sizing Matrix

| Deployment Tier | Workload / Concurrent Users | CPU | Memory (RAM) | Storage | Network |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Development** | Single developer / sandbox | 2 vCPU | 4 GB | 20 GB SSD | Localhost |
| **Production Tier 1** | 1 – 15 concurrent users | 2 vCPU | 4 GB – 8 GB | 50 GB NVMe | 100 Mbps |
| **Production Tier 2** | 16 – 60 concurrent users | 4 vCPU | 16 GB | 150 GB NVMe | 1 Gbps |
| **Production Tier 3** | 60 – 250+ concurrent users | 8 – 16 vCPU | 32 GB – 64 GB | 500 GB NVMe (RAID) | 10 Gbps |

### Operating System Compatibility
- **Windows**: Windows 10, 11, or Windows Server 2022+ (x86_64, aarch64)
- **Linux**: Ubuntu 22.04 LTS+, Debian 12+, RHEL 9+, Alpine Linux 3.19+ (x86_64, aarch64)
- **macOS**: macOS 13+ (Ventura, Sonoma, Sequoia - Apple Silicon M1/M2/M3 & Intel)

---

## 📦 Required Software Components

### 1. Rust Compiler Toolchain (1.80+)
BOS backend engines and compiler tooling are written in safe Rust (2024 edition).

- **Windows**: Install using [rustup-init.exe](https://rustup.rs/) (Select default option 1 with MSVC toolchain).
- **Linux / macOS**:
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  source $HOME/.cargo/env
  ```
- Verify:
  ```bash
  rustc --version
  cargo --version
  ```

### 2. Node.js & npm (20 LTS+)
The Developer Studio and Svelte 5 frontend runtime require Node.js 20 LTS or later.

- Download from [nodejs.org](https://nodejs.org/) or install via package manager (`fnm`, `nvm`, `winget`, `apt`):
  ```bash
  # Example via winget on Windows
  winget install OpenJS.NodeJS.LTS
  ```
- Verify:
  ```bash
  node -v
  npm -v
  ```

### 3. PostgreSQL Database Server (14+)
BOS requires PostgreSQL 14, 15, 16, 17, or 18+.

- **Authentication**: Enable password authentication (MD5 or SCRAM-SHA-256) in `pg_hba.conf`.
- **Default Database**: Ensure the maintenance database `postgres` is reachable with administrative rights for `create-db` execution.
- **Connection URL Format**:
  ```
  postgres://<username>:<password>@<host>:<port>/<database_name>
  ```
  *(e.g., `postgres://postgres:postgres@localhost:5432/postgres`)*

---

## 🔍 Verification of Environment & CLI Installation

### 1. Build and Install `bosc` Direct CLI
Install `bosc` directly to your Cargo bin directory (so `bosc` is available anywhere in your terminal):

```powershell
cargo install --path crates/bosc-cli
```

Verify that `bosc` is directly available:
```powershell
bosc --version
```

### 2. Verify Rust Crates
Run the workspace check:
```powershell
cargo check --workspace
```

### 3. Install Frontend Dependencies
```powershell
cd ui
npm install
cd ..
```

Your system is now ready for production or development use.
