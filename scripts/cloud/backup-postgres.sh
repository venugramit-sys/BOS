#!/bin/sh
# ==============================================================================
# Automated PostgreSQL Backup & Offsite Indian Cloud Replication Script
# Supports: AWS S3 (Mumbai ap-south-1), E2E Networks EOS, DigitalOcean BLR1 Spaces
# ==============================================================================

set -eu

TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
BACKUP_DIR="/backups"
BACKUP_FILE="${BACKUP_DIR}/bos_backup_${PGDATABASE}_${TIMESTAMP}.dump.gz"
RETENTION_DAYS=${BACKUP_RETENTION_DAYS:-30}

echo "[$(date -Iseconds)] [BOS BACKUP] Starting database dump for: ${PGDATABASE}..."
mkdir -p "${BACKUP_DIR}"

# Execute compressed custom format dump (high speed, parallel restore compatible)
PGPASSWORD="${PGPASSWORD}" pg_dump \
    -h "${PGHOST}" \
    -p "${PGPORT}" \
    -U "${PGUSER}" \
    -d "${PGDATABASE}" \
    -Fc \
    | gzip > "${BACKUP_FILE}"

BACKUP_SIZE=$(ls -lh "${BACKUP_FILE}" | awk '{print $5}')
echo "[$(date -Iseconds)] [BOS BACKUP] Backup created successfully: ${BACKUP_FILE} (Size: ${BACKUP_SIZE})"

# Prune local backups older than retention policy
echo "[$(date -Iseconds)] [BOS BACKUP] Pruning local backups older than ${RETENTION_DAYS} days..."
find "${BACKUP_DIR}" -type f -name "bos_backup_*.dump.gz" -mtime +"${RETENTION_DAYS}" -exec rm -f {} \;

# Offsite Replication to S3-compatible Object Storage (if configured)
if [ -n "${S3_ACCESS_KEY:-}" ] && [ -n "${S3_SECRET_KEY:-}" ] && [ -n "${S3_BUCKET:-}" ]; then
    echo "[$(date -Iseconds)] [BOS BACKUP] Replicating to Indian Cloud Object Storage (${S3_BUCKET})..."
    
    export AWS_ACCESS_KEY_ID="${S3_ACCESS_KEY}"
    export AWS_SECRET_ACCESS_KEY="${S3_SECRET_KEY}"
    
    if [ -n "${S3_ENDPOINT:-}" ]; then
        aws --endpoint-url "${S3_ENDPOINT}" s3 cp "${BACKUP_FILE}" "s3://${S3_BUCKET}/backups/$(basename "${BACKUP_FILE}")"
    else
        aws s3 cp "${BACKUP_FILE}" "s3://${S3_BUCKET}/backups/$(basename "${BACKUP_FILE}")"
    fi
    
    echo "[$(date -Iseconds)] [BOS BACKUP] Offsite replication complete!"
else
    echo "[$(date -Iseconds)] [BOS BACKUP] S3 credentials not provided. Local backup retained only."
fi

echo "[$(date -Iseconds)] [BOS BACKUP] Backup routine finished successfully."
