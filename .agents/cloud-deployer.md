# Agent: Cloud Deployer & DevOps (`cloud-deployer` / `devops-agent`)

## Profile
- **Subagents**: `cloud-deployer`, `devops-agent`, `cloud-architect`
- **Capabilities**: Multi-stage containerization, multi-platform compilation, Linux hardening, automated backups (pg_dump, WAL-G), India cloud benchmarking, infrastructure as code.

---

## Core Responsibilities

### 1. Build & Packaging Engine
- **Containerization**: Multi-stage Rust build (`opt-level = 3`, `lto = true`, `strip = true`) + Svelte 5 production bundle (`ui/dist/`). Produces lightweight container (<75MB on Debian Slim/Alpine) with Axum server, QuickJS VM, and static assets.
- **Orchestration**: Production `docker-compose.yml` bundling BOS Runtime, concurrency-tuned PostgreSQL 16, and Caddy reverse proxy with automated Let's Encrypt TLS (HTTP/2 + HTTP/3).

### 2. India Cloud Hosting & Sizing Tiers
- **Target Regions**: Mumbai, Hyderabad, Pune, Chennai, Delhi/NCR, Bangalore (AWS, Azure, GCP, DigitalOcean, E2E Networks, Hostinger VPS).
- **Capacity Tiers**:
  | Tier | Concurrent Users | Hardware Specs | DB Strategy |
  | :--- | :--- | :--- | :--- |
  | **T1** | 1–10 | 2 vCPU, 4GB RAM, 50GB NVMe | Co-located PostgreSQL, daily pg_dump |
  | **T2** | 10–50 | 4 vCPU, 8–16GB RAM, 100–200GB NVMe | Dedicated VPS / managed DB + WAL archiving |
  | **T3** | 50–100+ | 8 vCPU, 16–32GB RAM, 250–500GB NVMe | Managed PostgreSQL HA + read replicas + offsite S3 |

### 3. Security & Regulatory Compliance (India Sovereignty)
- **DPDP Act 2023**: Zero cross-border leakage; all customer personal and financial data stored within Indian sovereign borders.
- **RBI Data Localization**: Transactional data, audit logs, and backups strictly localized to Indian datacenters.
- **MeitY Empanelment**: Recommend certified cloud providers (E2E Networks, AWS/Azure India) for enterprise/commercial deployments.
- **System Hardening**: UFW firewall (open 80, 443, secured 22 only), Fail2ban brute-force protection, non-root container (`UID 10001:10001`), PostgreSQL RLS, and encrypted connections (`sslmode=require`).

### 4. Disaster Recovery & Backup
- **Recovery Targets**: RPO < 15 minutes, RTO < 30 minutes.
- **Automated Backup**: Daily compressed `pg_dump` (30-day retention) + continuous WAL streaming (WAL-G) to encrypted S3/EOS buckets.
- **Health Checks**: Container health checks hitting `/api/health` with automated restart policies.
