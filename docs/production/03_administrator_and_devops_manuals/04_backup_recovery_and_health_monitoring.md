# Volume 3: Administrator & DevOps Manuals
## 04. Backup, Recovery & Health Monitoring

Operational resilience requires robust backup pipelines, point-in-time recovery (PITR), and automated health checks. This guide details operational runbooks for enterprise deployments.

---

## 💾 Automated Backup Strategies

### 1. Daily Logical Snapshot (`pg_dump`)
Run daily compressed backups of each tenant database:

```bash
#!/usr/bin/env bash
set -euo pipefail

BACKUP_DIR="/var/backups/bos"
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
DATABASE="bos_production"
FILENAME="${BACKUP_DIR}/${DATABASE}_${TIMESTAMP}.dump"

mkdir -p "${BACKUP_DIR}"

# Run custom format compressed dump
pg_dump -U postgres -h localhost -Fc "${DATABASE}" > "${FILENAME}"

# Retain 30 days of snapshots
find "${BACKUP_DIR}" -type f -name "*.dump" -mtime +30 -delete

echo "Backup successful: ${FILENAME}"
```

### 2. Continuous Archiving & Point-In-Time Recovery (PITR)
For enterprise Tier 2/3 production, enable Write-Ahead Log (WAL) archiving in `postgresql.conf`:
```ini
wal_level = replica
archive_mode = on
archive_command = 'pgbackrest --stanza=bos archive-push %p'
```
This guarantees a Recovery Point Objective (RPO) of < 15 minutes.

---

## 🔄 Disaster Recovery: Restoring a Database

To restore from a backup snapshot:

```bash
# 1. Terminate active server nodes
sudo systemctl stop bos-node

# 2. Re-create database and restore snapshot
dropdb -U postgres -h localhost bos_production
createdb -U postgres -h localhost bos_production
pg_restore -U postgres -h localhost -d bos_production /var/backups/bos/bos_production_20261026_020000.dump

# 3. Verify kernel and run check
bosc node check --config /etc/bos/production.toml

# 4. Restart server node
sudo systemctl start bos-node
```

---

## 🩺 Health Probes & Monitoring

The BOS backend exposes lightweight endpoints for container orchestrators (Kubernetes, Docker Swarm, Nomad) and monitoring systems (Prometheus, Datadog):

### 1. Liveness Probe (`GET /health`)
- Returns HTTP 200 if the Axum HTTP server is responsive:
  ```json
  {
    "status": "healthy",
    "version": "0.1.0",
    "uptime_seconds": 84201
  }
  ```

### 2. Readiness Probe (`GET /health/ready`)
- Probes database connection pool status, system kernel tables, and NAS worker status:
  ```json
  {
    "status": "ready",
    "database": {
      "connected": true,
      "active_connections": 4,
      "max_connections": 50,
      "latency_ms": 1.2
    },
    "nas_worker": {
      "active": true,
      "queued_jobs": 0
    }
  }
  ```
