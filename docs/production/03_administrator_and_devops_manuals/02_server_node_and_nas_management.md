# Volume 3: Administrator & DevOps Manuals
## 02. Server Node & NAS Management

BOS utilizes configurable **Server Nodes** that combine the HTTP/REST API gateway, session management, static asset serving, and the **NAS (Navision Application Server Equivalent) Worker Daemon** for background jobs.

---

## 🖥️ Server Node Profile Architecture

Nodes are configured via TOML profiles stored in `nodes/` or at the workspace root (`bos-node.toml`).

```
┌────────────────────────────────────────────────────────┐
│ BOS Server Node Process (bosc node run)                │
├────────────────────────────┬───────────────────────────┤
│ Axum 0.8 HTTP API Server   │ NAS Background Worker     │
│ - Port: 3000 (REST / SSE)  │ - Scheduled Jobs & Cron   │
│ - Session Token Validation │ - Outbox Event Dispatch   │
│ - Dynamic Page Kit API     │ - Batch Posting Engine    │
├────────────────────────────┴───────────────────────────┤
│ SQLx Database Connection Pool (PostgreSQL 14+)         │
└────────────────────────────────────────────────────────┘
```

---

## 🛠️ Operational Commands

### 1. Initializing a Profile
Generate a documented template:
```powershell
bosc node init --output nodes/production.toml
```

### 2. Validating Configuration & Connectivity
Verify file syntax and database connectivity before starting services:
```powershell
bosc node check --config nodes/production.toml
```

### 3. Launching the Node
Start the node:
```powershell
bosc node run --config nodes/production.toml
```

---

## ⏱️ NAS Worker Daemon Configuration

The NAS Worker runs in the background of the server node, processing asynchronous queues and scheduled jobs without blocking HTTP request threads.

### Configuration in `bos-node.toml`:
```toml
[nas_worker]
# Enable or disable the background worker on this specific node
enabled = true

# Polling frequency for queue checks (milliseconds)
poll_interval_ms = 1000

# Maximum batch size per poll
batch_size = 50

# Maximum concurrent asynchronous task threads
max_concurrent_jobs = 8

# Maximum execution time before a job is marked timed out (seconds)
job_timeout_seconds = 300
```

> [!TIP]
> In high-scale multi-node deployments, you can deploy dedicated API nodes with `enabled = false` in `[nas_worker]` behind a load balancer, and run a separate dedicated NAS batch node with `enabled = true`.

---

## 🐧 Running as a Linux Systemd Service

Create `/etc/systemd/system/bos-node.service`:

```ini
[Unit]
Description=BOS Enterprise Server Node
After=network.target postgresql.service

[Service]
Type=simple
User=bos
Group=bos
WorkingDirectory=/opt/bos
ExecStart=/usr/local/bin/bosc node run --config /etc/bos/production.toml
Restart=always
RestartSec=5
LimitNOFILE=65536
Environment=RUST_LOG=info,bosc_runtime=debug

[Install]
WantedBy=multi-user.target
```

Enable and start:
```bash
sudo systemctl daemon-reload
sudo systemctl enable --now bos-node
sudo journalctl -u bos-node -f
```
