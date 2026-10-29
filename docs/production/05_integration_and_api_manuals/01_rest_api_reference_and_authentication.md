# Volume 5: Integration & API Manuals
## 01. REST API Reference & Authentication

The BOS Backend provides an enterprise REST API powered by Axum 0.8 with uniform JSON envelopes, session authentication, and Optimistic Concurrency Control.

---

## 🔐 Authentication & Session Headers

Every authenticated request requires a Bearer session token:

```http
Authorization: Bearer <SessionToken>
X-Tenant-ID: 018f4a12-8800-7214-9bf2-b430c5e7b411
X-Company-ID: 018f4a12-8800-7214-9bf2-b430c5e7b499
```

### Logging In (`POST /api/auth/login`)
```http
POST /api/auth/login
Content-Type: application/json

{
  "username": "admin",
  "password": "Password123!"
}
```

**Response (HTTP 200 OK):**
```json
{
  "token": "bos_sess_8f3a9e2c4d1b7a6e5f...",
  "user": {
    "id": "018f4a12-8800-7214-9bf2-b430c5e7b411",
    "username": "admin",
    "roles": ["SystemAdmin"]
  },
  "default_tenant": "018f4a12-8800-7214-9bf2-b430c5e7b411",
  "default_company": "018f4a12-8800-7214-9bf2-b430c5e7b499"
}
```

---

## 📊 Universal Record CRUD Endpoints

All business tables exposed in packages can be queried and modified via standard endpoints:

### 1. List Records (`GET /api/data/:table_name`)
Query with sorting, filtering, and pagination:
```http
GET /api/data/Customer?limit=20&offset=0&order=name.asc&status=eq.Active
```

**Response:**
```json
{
  "data": [
    {
      "id": "018f4a12-9900-7214-9bf2-b430c5e7b4aa",
      "customer_no": "CUST-001",
      "name": "Acme Industrial Corp",
      "credit_limit": "50000.0000",
      "balance": "14250.0000",
      "status": "Active",
      "_system_version": 4
    }
  ],
  "total": 1,
  "limit": 20,
  "offset": 0
}
```

### 2. Get Record by ID (`GET /api/data/:table_name/:id`)
```http
GET /api/data/Customer/018f4a12-9900-7214-9bf2-b430c5e7b4aa
```

### 3. Create Record (`POST /api/data/:table_name`)
```http
POST /api/data/Customer
Content-Type: application/json

{
  "name": "New Horizons Logistics",
  "credit_limit": "15000.0000",
  "status": "Active"
}
```

### 4. Update Record (`PATCH /api/data/:table_name/:id`)
Enforces Optimistic Concurrency Control via `_system_version`:
```http
PATCH /api/data/Customer/018f4a12-9900-7214-9bf2-b430c5e7b4aa
Content-Type: application/json

{
  "credit_limit": "20000.0000",
  "_system_version": 4
}
```
If `_system_version` in the database is no longer 4, the API returns **HTTP 409 Conflict**.

### 5. Delete Record (`DELETE /api/data/:table_name/:id`)
```http
DELETE /api/data/Customer/018f4a12-9900-7214-9bf2-b430c5e7b4aa
```

---

## ⚡ Codeunit RPC Execution (`POST /api/codeunit/:codeunit_name/:method`)

Execute custom business service methods:
```http
POST /api/codeunit/PurchasePost/post
Content-Type: application/json

{
  "orderId": "018f4a12-8800-7214-9bf2-b430c5e7b411"
}
```
