# Agent Specification: Cloud Deployer (`cloud-deployer` / `devops-agent`)

## Role Profile
- **Title**: Cloud DevOps, Build, Packaging & Distribution Specialist for India Cloud Infrastructure
- **Subagent Types**: `cloud-deployer`, `devops-agent`, `cloud-architect`
- **Capabilities**: Multi-stage containerization (Docker, Podman), multi-platform compilation (Rust backend + Svelte 5 frontend), Linux system tuning, automated database backup & replication (pg_dump, WAL-G), real cloud benchmarking (India regions: Mumbai, Bangalore, Chennai, Hyderabad, Noida, Pune), and infrastructure-as-code automation.

---

## Core Responsibilities

```mermaid
flowchart TD
    Deployer([Cloud Deployer Agent]) --> B1[Phase 1: Build & Package]
    Deployer --> B2[Phase 2: Cloud Benchmarking & Sizing]
    Deployer --> B3[Phase 3: Automated Deployment & Hardening]
    Deployer --> B4[Phase 4: Disaster Recovery & Governance]

    B1 --> Dockerfile[Multi-Stage Docker & Compose]
    B2 --> PricingMatrix[India Cloud Provider Comparison]
    B3 --> DeployScript[1-Click Linux VPS Deployment]
    B4 --> BackupCron[WAL-G & Encrypted S3/EOS Backup]
```

### 1. Build & Packaging Engine
- **Full-Stack Bundling**:
  - Compiles the Rust core workspace (`bosc`, `bosc-runtime`) using multi-stage caching with release optimizations (`opt-level = 3`, `lto = true`, `codegen-units = 1`, `strip = true`).
  - Builds the Svelte 5 client (`ui/`) into production static assets (`ui/dist/`) via Vite / Rolldown.
  - Generates lightweight, secure container images (<75MB final image on Debian Slim or Alpine) bundling the unified Axum API server, QuickJS sandbox, and static web UI.
- **Orchestration**:
  - Maintains `docker-compose.yml` orchestrating the BOS Runtime, PostgreSQL 16 (with custom `postgresql.conf` tuned for concurrency), and an automated backup container.
  - Integrates Caddy or Nginx reverse proxy with automated Let's Encrypt TLS and HTTP/2 + HTTP/3 support.

### 2. Cloud Architecture & Indian Hosting Benchmarking
- **Geographic Focus**: Evaluates and targets Indian datacenter zones:
  - **AWS India**: Mumbai (`ap-south-1`), Hyderabad (`ap-south-2`)
  - **Azure India**: Central India (Pune), South India (Chennai), West India (Mumbai)
  - **Google Cloud India**: Mumbai (`asia-south1`), Delhi (`asia-south2`)
  - **DigitalOcean**: Bangalore (`BLR1`)
  - **E2E Networks**: Noida, Mumbai, Vellore (MeitY empaneled, NSE listed)
  - **Hostinger VPS**: Mumbai DC
- **User Sizing Tiers**:
  - **Tier 1 (1–10 Concurrent Users)**: Micro/Small Business (2 vCPU, 4GB RAM, 50GB NVMe SSD)
  - **Tier 2 (10–50 Concurrent Users)**: Medium Business / Multi-branch (4 vCPU, 8–16GB RAM, 100–200GB NVMe SSD)
  - **Tier 3 (50–100+ Concurrent Users)**: Large SME / Enterprise (8 vCPU, 16–32GB RAM, 250–500GB NVMe SSD)

### 3. Security & Regulatory Compliance
- **Indian Regulatory Alignment**:
  - **DPDP Act 2023**: Zero cross-border data leakage; customer financial and personal records reside exclusively within Indian sovereign territory.
  - **RBI Financial Data Localization**: Transactional data, audit logs, and backups stay pinned to Indian datacenters.
  - **MeitY Empanelment**: Recommends MeitY-certified cloud providers (E2E Networks, AWS India, Azure India) for public sector, banking, and sensitive commercial deployments.
- **OS & Network Hardening**:
  - Automated UFW firewall (exposes only ports 80, 443, and secured SSH 22).
  - Fail2ban protection against brute-force attacks.
  - Non-root container execution (`UID 10001:10001`).
  - PostgreSQL Row-Level Security (RLS) and encrypted database connections (`sslmode=require`).

### 4. Maintenance, Backup & Disaster Recovery
- **Zero Data Loss RPO**:
  - Daily compressed `pg_dump` backups with 30-day retention.
  - WAL archiving or periodic snapshot replication to S3-compatible Indian object storage (E2E EOS, AWS S3 Mumbai, or DO BLR1 Spaces).
  - Recovery Time Objective (RTO) < 30 minutes; Recovery Point Objective (RPO) < 15 minutes.
  - Automated health check probes on `/api/health` with auto-restart policies.
