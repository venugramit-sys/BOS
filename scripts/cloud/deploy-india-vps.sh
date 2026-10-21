#!/usr/bin/env bash
# ==============================================================================
# 1-Click Production VPS Provisioning Script for BOS
# Target OS: Ubuntu 22.04 / 24.04 LTS (x86_64 or aarch64)
# Target Datacenters: Mumbai, Bengaluru, Chennai, Hyderabad, Noida, Pune
# ==============================================================================

set -euo pipefail

echo "===================================================================="
echo "    BOS Production VPS Provisioner (Indian Cloud Infrastructure)    "
echo "===================================================================="

# Check root privileges
if [ "$EUID" -ne 0 ]; then
  echo "[-] Please run as root (use sudo)."
  exit 1
fi

DOMAIN_NAME="${1:-}"
ACME_EMAIL="${2:-}"

if [ -z "$DOMAIN_NAME" ] || [ -z "$ACME_EMAIL" ]; then
  echo "Usage: sudo ./deploy-india-vps.sh <DOMAIN_NAME> <ACME_EMAIL>"
  echo "Example: sudo ./deploy-india-vps.sh erp.mycompany.in admin@mycompany.in"
  exit 1
fi

echo "[+] Step 1: Updating system packages and dependencies..."
apt-get update && apt-get upgrade -y
apt-get install -y --no-install-recommends \
    curl \
    git \
    ufw \
    fail2ban \
    htop \
    ca-certificates \
    gnupg \
    lsb-release \
    tzdata

echo "[+] Step 2: Configuring Timezone to Asia/Kolkata (IST)..."
timedatectl set-timezone Asia/Kolkata

echo "[+] Step 3: Hardening Linux Kernel sysctl parameters for ERP workload..."
cat << 'EOF' > /etc/sysctl.d/99-bos-performance.conf
# Maximum open files and socket connections
fs.file-max = 2097152
net.core.somaxconn = 65535
net.ipv4.tcp_max_syn_backlog = 8192
net.ipv4.ip_local_port_range = 10240 65535

# Virtual Memory management
vm.overcommit_memory = 1
vm.swappiness = 10
vm.dirty_background_ratio = 5
vm.dirty_ratio = 10
vm.max_map_count = 262144
EOF
sysctl --system

echo "[+] Step 4: Installing Docker Engine & Docker Compose Plugin..."
if ! command -v docker &> /dev/null; then
  install -m 0755 -d /etc/apt/keyrings
  curl -fsSL https://download.docker.com/linux/ubuntu/gpg | gpg --dearmor -o /etc/apt/keyrings/docker.gpg
  chmod a+r /etc/apt/keyrings/docker.gpg

  echo \
    "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/ubuntu \
    $(lsb_release -cs) stable" | tee /etc/apt/sources.list.d/docker.list > /dev/null

  apt-get update
  apt-get install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin
  systemctl enable --now docker
else
  echo "[+] Docker is already installed."
fi

echo "[+] Step 5: Configuring Security Hardening (UFW & Fail2ban)..."
ufw default deny incoming
ufw default allow outgoing
ufw allow 22/tcp comment 'SSH'
ufw allow 80/tcp comment 'HTTP ACME'
ufw allow 443/tcp comment 'HTTPS Caddy'
ufw --force enable

systemctl enable --now fail2ban

echo "[+] Step 6: Generating Secure Production Secrets..."
POSTGRES_PASS=$(openssl rand -hex 24)
BOS_SECRET=$(openssl rand -hex 32)

cat << EOF > .env
DOMAIN_NAME=${DOMAIN_NAME}
ACME_EMAIL=${ACME_EMAIL}
POSTGRES_USER=bos_admin
POSTGRES_PASSWORD=${POSTGRES_PASS}
POSTGRES_DB=bos_prod
BOS_JWT_SECRET=${BOS_SECRET}
TZ=Asia/Kolkata
EOF
chmod 600 .env

echo "[+] Step 7: Starting BOS Application Stack..."
docker compose pull || true
docker compose up -d --build

echo "[+] Step 8: Waiting for Health Check..."
for i in {1..30}; do
  if docker compose ps bos-app | grep -q "healthy"; then
    echo "[+] BOS stack is up and healthy!"
    break
  fi
  echo "    Waiting for services to become healthy ($i/30)..."
  sleep 3
done

echo "===================================================================="
echo "    BOS DEPLOYMENT COMPLETE!                                        "
echo "    Domain: https://${DOMAIN_NAME}                                  "
echo "    Health API: https://${DOMAIN_NAME}/api/health                   "
echo "    Database Credentials saved in .env (restricted permissions)     "
echo "===================================================================="
