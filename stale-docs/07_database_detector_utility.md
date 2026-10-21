# Database Provider Detection Engine

> **Module:** `bosc-runtime::detector` & `bosc detect-db`  
> **Status:** 100% Implemented & Verified  
> **Timestamp:** 2026-10-06  

---

## 1. Overview & Capability

The BOS database detection utility enables developers and deployment systems to automatically discover and probe locally installed and running database engines. While **PostgreSQL** is the default production dialect, the detection engine probes all major enterprise database engines and cache systems:

1. **PostgreSQL** (Default primary OLTP & ERP engine, RLS, advanced schemas)
2. **MySQL / MariaDB** (Alternative relational store)
3. **Microsoft SQL Server** (Enterprise alternative)
4. **ClickHouse** (High-throughput analytical store for large ledger aggregations)
5. **Redis** (In-memory cache & event outbox stream)
6. **MongoDB** (Unstructured document store)
7. **SQLite** (Zero-setup embedded database for local edge/offline deployment)

---

## 2. Detection Methodology

The detection engine uses a two-pronged, zero-overhead probing strategy:
1. **TCP Socket Probe (`TcpStream::connect_timeout`):** Performs non-blocking, sub-second active socket probes on standard database ports (`5432`, `3306`, `1433`, `8123`, `6379`, `27017`).
2. **Filesystem Binary Resolution:** Inspects standard system installation directories (e.g. `C:\Program Files\PostgreSQL\18\bin\psql.exe`, `/usr/bin/psql`) to identify installed services even if currently stopped.

---

## 3. Live Detection Scan Output

Execution of `cargo run -p bosc-cli -- detect-db` produces:

```
Scanning local system for installed and running database providers...

DATABASE PROVIDER                STATUS     ENDPOINT         DETAILS                         
-----------------------------------------------------------------------------------------------
PostgreSQL (Default BOS Engine)  RUNNING    127.0.0.1:5432   Active and listening on TCP 5432 (Ready for BOS)
    -> Binary path: C:\Program Files\PostgreSQL\18\bin\psql.exe
MySQL / MariaDB                  OFFLINE    127.0.0.1:3306   Not detected                    
Microsoft SQL Server             OFFLINE    127.0.0.1:1433   Not detected                    
ClickHouse (Analytical DB)       OFFLINE    127.0.0.1:8123   Not detected                    
Redis (Cache & Message Bus)      OFFLINE    127.0.0.1:6379   Not detected                    
MongoDB (Document Store)         OFFLINE    127.0.0.1:27017  Not detected                    
SQLite (Embedded DB)             RUNNING    embedded         Embedded driver available (Zero setup required)

BOS Default Connection String:
  postgres://postgres:postgres@localhost:5432/bos_dev
```
