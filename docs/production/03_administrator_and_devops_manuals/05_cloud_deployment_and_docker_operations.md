# Volume 3: Administrator & DevOps Manuals
## 05. Cloud Deployment & Docker Operations

This guide provides instructions for deploying BOS into containerized environments (Docker Compose) and cloud hosting providers with strict compliance for Indian data localization regulations.

---

## 🐳 Docker Deployment with Compose

The repository includes a production-ready `Dockerfile` and `docker-compose.yml`.

### Architecture of the Stack
```
┌────────────────────────────────────────────────────────┐
│ Caddy Reverse Proxy (Automatic Let's Encrypt TLS)      │
│ Ports: 80 / 443                                        │
└──────────────────────────┬─────────────────────────────┘
                           │
┌──────────────────────────▼─────────────────────────────┐
│ BOS Server Node Container (`bosc node run`)            │
│ Port: 3000 (Internal)                                  │
└──────────────────────────┬─────────────────────────────┘
                           │
┌──────────────────────────▼─────────────────────────────┐
│ PostgreSQL 16 Container (NVMe Persistent Volume)       │
│ Port: 5432 (Internal Network Only)                     │
└────────────────────────────────────────────────────────┘
```

### Launching the Stack
1. Clone the repository and configure your environment variables:
   ```bash
   cp .env.example .env
   ```
2. Build and launch containers in the background:
   ```bash
   docker compose up -d --build
   ```
3. Inspect container logs:
   ```bash
   docker compose logs -f bos-node
   ```

---

## 🇮🇳 Indian Data Localization & Compliance (DPDP Act 2023)

For enterprises operating in India or handling Indian citizen data, BOS deployment architectures must adhere to the **Digital Personal Data Protection (DPDP) Act 2023** and **RBI Data Localization Guidelines**:

### 1. Data Center Locations
Deploy all PostgreSQL primary databases and WAL archive buckets in Indian data centers:
- **AWS**: Asia Pacific (Mumbai) `ap-south-1` or (Hyderabad) `ap-south-2`
- **Google Cloud**: `asia-south1` (Mumbai) or `asia-south2` (Delhi)
- **Azure**: Central India (Pune), South India (Chennai), West India (Mumbai)
- **Indian VPS Providers**: E2E Networks (Noida/Mumbai), Yotta NM1 (Navi Mumbai)

### 2. Encryption Standards
- **Data in Transit**: Enforce TLS 1.3 with AES-256-GCM / ChaCha20-Poly1305 ciphers.
- **Data at Rest**: LUKS encrypted NVMe storage volumes and PostgreSQL tablespace encryption.

---

## 🔒 Production Security Hardening Checklist

- [ ] Disable default PostgreSQL administrative credentials (`postgres:postgres`).
- [ ] Bind database ports only to private loopback/Docker networks; never expose port 5432 to the public internet.
- [ ] Configure automatic daily encrypted backups to an S3-compatible bucket located within Indian jurisdiction.
- [ ] Restrict `CORS` allowed origins in `bos-node.toml` strictly to your production domain name.
- [ ] Ensure non-root execution inside Docker containers (`USER bos`).
