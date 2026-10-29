# Enterprise Use Cases & Reference Implementations

> **Document ID:** `BOS-DOC-12`  
> **Source Verification:** `examples/*`, `test_*.ps1`, `crates/bosc-runtime/tests/*`  
> **Target Audience:** Solution Architects, Implementation Engineers, QA Specialists

---

## 1. Overview

This document presents three production-grade enterprise reference implementations directly verified by the BOS automated test suites:
1. **Enterprise Identity, RBAC & Session Management (`bos.auth`)**
2. **Logistics & Fleet Dispatch Validator**
3. **Optimistic Concurrency Control, Row Locking & Referential Ledger Protection**

---

## 2. Use Case 1: Enterprise Identity & RBAC Package (`bos.auth`)

Directly derived from [`examples/security_module.bos`](file:///d:/Work%20Studio/Bos/examples/security_module.bos) and verified in [`test_security_api.ps1`](file:///d:/Work%20Studio/Bos/test_security_api.ps1).

### 2.1. Business Problem
An enterprise requires a secure, auditable authentication and role-based access control (RBAC) foundation providing:
- Global user accounts across companies (`dataPerCompany = false`).
- Zero plaintext passwords, enforcing Argon2id password hashing.
- Role-based user assignments.
- Revocable session token tracking with IP binding.
- Service codeunit for registration validation and session probing.

### 2.2. BOS-Script Implementation

#### 1. Tables (`User`, `Role`, `UserRole`, `SessionToken`)
```typescript
table User {
    caption = "Security User Master";
    tableType = Normal;
    dataPerCompany = false;

    username: String(50) {
        notNull = true;
        unique = true;
    };
    passwordHash: String(255) {
        notNull = true;
    };
    email: String(100) {
        notNull = true;
    };
    fullName: String(100);
    isActive: Boolean {
        default = true;
    };
    failedAttempts: Integer {
        default = 0;
    };

    primaryKey(username);
}

table Role {
    caption = "Security Role";
    tableType = Normal;
    dataPerCompany = false;

    code: String(30) {
        notNull = true;
        unique = true;
    };
    name: String(100);
    description: String(255);

    primaryKey(code);
}

table UserRole {
    caption = "User Role Mapping";
    tableType = Normal;
    dataPerCompany = false;

    username: String(50) { notNull = true; };
    roleCode: String(30) { notNull = true; };

    primaryKey(username, roleCode);
}

table SessionToken {
    caption = "Active Session Tokens";
    tableType = Normal;
    dataPerCompany = false;

    token: String(128) {
        notNull = true;
        unique = true;
    };
    username: String(50) { notNull = true; };
    ipAddress: String(45);
    isRevoked: Boolean {
        default = false;
    };

    primaryKey(token);
}
```

#### 2. Business Logic Companions (`UserLogic`, `RoleLogic`)
```typescript
tablelogic UserLogic for User {
    onInsert(rec, ctx) {
        if (!rec.username || rec.username.trim().length === 0) {
            throw new Error("Field 'username' is required");
        }
        if (!rec.email || rec.email.trim().length === 0) {
            throw new Error("Field 'email' is required");
        }
        rec.email = rec.email.toLowerCase().trim();

        // Enforce Argon2id hashing
        if (rec.password) {
            rec.passwordHash = String(rec.password).startsWith("$argon2")
                ? String(rec.password)
                : ctx.crypto.argon2Hash(rec.password);
            delete rec.password;
        } else if (rec.passwordHash && !String(rec.passwordHash).startsWith("$argon2")) {
            rec.passwordHash = ctx.crypto.argon2Hash(rec.passwordHash);
        }

        ctx.audit.log("Security identity master record inserted");
    }

    onDelete(rec, ctx) {
        if (rec.username === "admin") {
            throw new Error("Cannot delete master administrator account");
        }
        ctx.audit.log("Security identity master record deleted");
    }
}
```

#### 3. Service Codeunit (`AuthManager`)
```typescript
codeunit AuthManager {
    ValidateUserRegistration(username: String, email: String) {
        const u = String(username).trim();
        const e = String(email).toLowerCase().trim();

        if (User.exists({ username: u })) {
            throw new Error(`Username '${u}' is already registered`);
        }
        if (User.exists({ email: e })) {
            throw new Error(`Email '${e}' is already registered`);
        }

        ctx.audit.log(`User registration parameters verified for '${u}'`);
        return { valid: true, username: u, email: e };
    }

    CheckTokenStatus(token: String) {
        const tokenKey = String(token).trim();
        const sessionRec = SessionToken.get(tokenKey);
        if (!sessionRec) {
            return { valid: false, reason: "NOT_FOUND" };
        }
        if (sessionRec.isRevoked === true) {
            return { valid: false, reason: "REVOKED", username: sessionRec.username };
        }

        const userRec = User.get(sessionRec.username);
        if (!userRec || userRec.isActive === false) {
            return { valid: false, reason: "USER_INACTIVE", username: sessionRec.username };
        }

        return { valid: true, username: sessionRec.username, ipAddress: sessionRec.ipAddress };
    }
}
```

### 2.3. Live API Verification Steps
1. **Create Security Role**:
   ```powershell
   Invoke-RestMethod -Uri "http://localhost:3000/api/entities/role" -Method Post -ContentType "application/json" -Body '{"code":"FIN_MGR","name":"Finance Manager"}'
   ```
2. **Create User**:
   ```powershell
   Invoke-RestMethod -Uri "http://localhost:3000/api/entities/user" -Method Post -ContentType "application/json" -Body '{"username":"john.doe","email":"john@enterprise.com","password":"SuperSecretPassword123!"}'
   ```
3. **Execute Registration Check via Codeunit**:
   ```powershell
   Invoke-RestMethod -Uri "http://localhost:3000/api/codeunits/AuthManager/run" -Method Post -ContentType "application/json" -Body '{"method":"ValidateUserRegistration","params":{"username":"john.doe","email":"john@enterprise.com"}}'
   ```
   *Result:* Automatically rejects with `"Username 'john.doe' is already registered"`.

---

## 3. Use Case 2: Logistics & Fleet Dispatch Validator

Directly derived from [`examples/dispatch_validator.bos`](file:///d:/Work%20Studio/Bos/examples/dispatch_validator.bos).

### 3.1. Business Problem
A logistics hub dispatches commercial cargo vehicles. Before a truck can be released, its vehicle identification number (VIN) and payload tonnage must be validated against fleet safety constraints.

### 3.2. BOS-Script Implementation
```typescript
table VehicleDispatch {
    caption = "Vehicle Dispatch Record";
    tableType = Document;
    dataPerCompany = true;

    id: UUID {
        default = auto();
    };
    vin: String(30) {
        notNull = true;
    };
    capacityTons: Decimal(10, 2);
    inspectionStatus: String(30);

    primaryKey(id);
}

codeunit FleetValidator {
    ValidateFleetDispatch(vin: String, capacityTons: Decimal) {
        validateRequired(vin);
        assert(capacityTons > 0, "Payload capacity must be greater than 0 tons");
        inspectionStatus = "Approved";
        logAudit("Vehicle dispatch parameters successfully verified by engine");
    }
}
```

### 3.3. Command-Line Execution
```powershell
# 1. Compile directly into PostgreSQL DDL
bosc build -i examples/dispatch_validator.bos -o dispatch.sql

# 2. Execute Codeunit procedure
bosc run -c FleetValidator -m ValidateFleetDispatch -p '{"vin":"1HGCR2F83HA123456","capacityTons":18.5}' -f examples/dispatch_validator.bos
```

---

## 4. Use Case 3: Concurrency Control, Locking & Referential Protection

Directly verified in [`test_advanced_features.ps1`](file:///d:/Work%20Studio/Bos/test_advanced_features.ps1).

### 4.1. Optimistic Concurrency Control (OCC)
In multi-user environments, two users modifying the same record simultaneously can cause "lost updates". BOS solves this with `_system_version`:

1. **User A** fetches `Customer` with `_system_version = 1`.
2. **User B** fetches `Customer` with `_system_version = 1`.
3. **User A** submits an update with `_system_version = 1`. The update succeeds and increments `_system_version` to `2`.
4. **User B** submits an update with `_system_version = 1`.
5. **Engine Response**: Immediately aborts with **HTTP 409 Conflict**, preventing User A's changes from being silently overwritten.

### 4.2. Pessimistic Row Locking
For batch processes or sequential numbering routines:
```http
POST /api/entities/customer/b3e64f89-c451-419b-a320-cb1067d5ce89/lock
Content-Type: application/json
```
The server executes `SELECT ... FOR UPDATE`, blocking concurrent transactions until the current transaction commits or rolls back.

### 4.3. Referential Deletion Protection
When a record has dependent ledger transactions, deleting the master record corrupts financial history.

In BOS:
```powershell
# Attempting to delete a Customer with linked CustomerLedgerEntry rows:
Invoke-RestMethod -Uri "http://localhost:3000/api/entities/customer/$customerId" -Method Delete
```
**Engine Response:**
The deletion is intercepted and blocked by the PostgreSQL referential trigger with an error:
`"Cannot delete entity: active referential entries exist in customer_ledger_entry"`.
