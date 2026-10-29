# Volume 1: Getting Started
## 01. 5-Minute Quickstart Guide

Get up and running with **BOS (Business Operating System)** on your local development machine in under 5 minutes.

---

## ⚡ Prerequisites Check

Ensure you have the following installed:
- **Rust Toolchain (1.80+)**: Run `rustc --version`
- **Node.js (20+) & npm**: Run `node -v` and `npm -v`
- **PostgreSQL (14+)**: Running locally on port `5432` with user `postgres` (or your preferred credentials)

> [!TIP]
> If you do not have PostgreSQL running locally, launch it instantly with Docker:
> ```powershell
> docker run --name bos-postgres -e POSTGRES_PASSWORD=postgres -p 5432:5432 -d postgres:16-alpine
> ```

---

## 🚀 Step 1: Detect Local Database Infrastructure

BOS includes an automatic database engine detector. Run the probe using the direct `bosc` CLI:

```powershell
bosc detect-db
```

**Expected Output:**
```
✓ PostgreSQL detected on 127.0.0.1:5432 (Ready for provisioning)
```

---

## 🛠️ Step 2: Create and Provision the BOS Database

Run `create-db` to automatically create the database, initialize the BOS System Kernel (`_bos_sys_info`, `_bos_meta_objects`, `_bos_tenants`, `_bos_companies`), and install default base security packages:

```powershell
bosc create-db --name bos_dev --cluster-url postgres://postgres:postgres@localhost:5432/postgres --strategy shared_rls
```

**What just happened?**
1. The PostgreSQL database `bos_dev` was created.
2. The BOS System Kernel tables were installed in a single atomic transaction.
3. Row-Level Security (RLS) policies were activated.
4. Default Tenant (`DEFAULT_TENANT`) and Default Company (`MAIN_COMPANY`) were registered.

---

## 💻 Step 3: Launch the Full Stack (Node + UI)

Launch the BOS Server Node (API server + NAS daemon) and the Svelte 5 Developer Studio with a single command:

```powershell
npm run dev
```

This starts:
- **BOS Backend Node API**: `http://localhost:3000`
- **Svelte 5 Developer Studio**: `http://localhost:5173`

> [!NOTE]
> Alternatively, run the backend node directly:
> ```powershell
> bosc node run --config nodes/dev.toml
> ```
> And in a separate terminal launch the UI:
> ```powershell
> cd ui
> npm run dev
> ```

---

## 🌐 Step 4: Access the Developer Studio

1. Open your web browser and navigate to:
   ```
   http://localhost:5173
   ```
2. You will be greeted by the **BOS Developer Studio**:
   - **Entity Inspector**: Inspect live database tables, columns, and FlowFields.
   - **Monaco Code Editor**: Test BOS-Script and TypeScript business logic in real time.
   - **Interactive API Studio**: Execute REST queries with 1-click test payloads.
   - **Event & Outbox Monitor**: View real-time business events and audit logs.

---

## 🎯 Next Steps

Congratulations! You have a fully operational BOS instance running on your machine.
- Continue to **[02. Installation & System Requirements](02_installation_and_system_requirements.md)** for production requirements.
- Explore **[Volume 2: Navigating the UI Studio](../02_end_user_manuals/01_navigating_the_ui_studio.md)** for a guided tour of the user interface.
- Read **[Volume 4: BOS Compiler CLI Complete Guide](../04_developer_manuals/01_bosc_cli_complete_guide.md)** to build your first `.bos` package.
