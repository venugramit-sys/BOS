# Enterprise Extensibility & Multi-Tier Customization Guide

> **Document:** `docs/15_enterprise_extensibility_and_customization_guide.md`  
> **Status:** Production Architecture & Developer Guide  
> **Audience:** End Users, Power Users, ERP Consultants, In-House Developers, and ISV Partners

---

## 1. Executive Summary & Design Philosophy

BOS enables customizations at every level of the organization without risking core system instability, upgrade friction, or data corruption:
* **The Golden Rule of Extensibility:** Upstream base code is **never modified directly**. All customizations exist as non-invasive extensions, declarative metadata, event subscribers, or isolated sandboxed plugins.
* **Upgrade Safety:** A base platform upgrade or patch will never overwrite or break custom fields, workflows, or UI layouts.

---

## 2. The 4 Customization Personas & Capabilities Matrix

```
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                        BOS MULTI-TIER CUSTOMIZATION PYRAMID                            │
├────────────────────┬─────────────────────────────┬─────────────────────────────────────┤
│ Persona            │ Allowed Customizations      │ Technology & Sandbox Boundary       │
├────────────────────┼─────────────────────────────┼─────────────────────────────────────┤
│ 1. End User        │ Column order, hidden fields,│ Declarative JSON Deltas             │
│                    │ frozen panes, saved views,  │ Stored in `_bos_user_personalization`│
│                    │ session WorkDate adjustment │ Zero code execution, 100% safe      │
├────────────────────┼─────────────────────────────┼─────────────────────────────────────┤
│ 2. Power User /    │ Excel-like Formula Columns, │ Generated SQL columns & Declarative │
│    Business Analyst│ Validation guardrail rules, │ rule engine with 1,000-fuel quota   │
│                    │ custom notification templates│ No procedural script allowed       │
├────────────────────┼─────────────────────────────┼─────────────────────────────────────┤
│ 3. ERP Developer   │ Table Extensions, Pages,    │ BOS-Script (TypeScript) triggers    │
│    / In-House IT   │ Codeunits, Number Series,   │ Sandboxed QuickJS VM (8MB RAM,      │
│                    │ Event Subscribers           │ 100k fuel, zero OS socket access)   │
├────────────────────┼─────────────────────────────┼─────────────────────────────────────┤
│ 4. ISV Partner     │ Heavy algorithms, Barcode/QR│ WebAssembly (`wasmi`, fuel limit)   │
│    Solution Vendor │ generation, AI connectors,  │ or Out-of-Process Sidecar Container │
│                    │ Bank/Tax integration APIs   │ (mTLS gRPC / Unix Domain Sockets)   │
└────────────────────┴─────────────────────────────┴─────────────────────────────────────┘
```

---

## 3. Tier 1: End-User Personalization & Session Customization

### 3.1 Session WorkDate Setting
Accountants and operations staff frequently back-date or forward-date entries to close monthly books. Instead of using server clock time, the user configures their personal **WorkDate**:

* **Configured in Settings:** `POST /api/v1/user/settings`
  ```json
  {
    "workDate": "2026-10-31",
    "timeZone": "Asia/Kolkata",
    "locale": "en-IN"
  }
  ```
* **Runtime Application:** All BOS-Script triggers access `ctx.user.workDate` for default posting dates:
  ```typescript
  rec.postingDate = rec.postingDate || ctx.user.workDate;
  ```

### 3.2 UI Grid & Form Layout Personalization
Users customize UI layouts using the drag-and-drop studio in the Svelte frontend. Changes are saved as non-invasive JSON deltas in `_bos_user_personalization`:

* **Stored Layout Delta (`_bos_user_personalization`):**
  ```json
  {
    "pageId": "CustomerList",
    "viewName": "HighValueAccounts",
    "layoutDelta": {
      "hiddenColumns": ["faxNo", "telexNo"],
      "columnOrder": ["customerNo", "name", "creditLimit", "balance"],
      "frozenPaneColumn": "name"
    },
    "pinnedFilters": [
      { "field": "balance", "operator": "gt", "value": 100000 },
      { "field": "status", "operator": "eq", "value": "Active" }
    ]
  }
  ```

---

## 4. Tier 2: Power-User No-Code Customizations

### 4.1 Formula Calculated Columns
Power users add computed fields to any table via the Admin Studio without writing procedural code:

```typescript
// Configured via Admin UI -> Added to Table Metadata:
column totalWithTax: Decimal(18, 2) {
    caption = "Total (Incl. Tax)";
    formula = "totalAmount * (1.0 + taxRate / 100.0)";
}
```
* **Compiler Lowering:** The compiler translates this directly into a native PostgreSQL Generated Column:
  ```sql
  ALTER TABLE "sales_header" 
  ADD COLUMN "total_with_tax" NUMERIC(18, 2) 
  GENERATED ALWAYS AS ("total_amount" * (1.0 + "tax_rate" / 100.0)) STORED;
  ```
* **Benefit:** Evaluated directly in PostgreSQL with zero CPU overhead in the backend application server.

### 4.2 Declarative Validation Rules
Guardrails created by managers to enforce operational policy:

```json
{
  "ruleId": "RULE_CORP_CREDIT_CAP",
  "targetTable": "Customer",
  "condition": "creditLimit > 250000 AND NOT isCorporateClient",
  "severity": "Error",
  "message": "Retail customers cannot be assigned a credit limit exceeding 250,000."
}
```

---

## 5. Tier 3: Developer Extensions in BOS-Script

### 5.1 Non-Invasive Table Extension (`tableextension`)
A developer adds custom fields and keys to the core `Customer` table without modifying the base package:

```typescript
// File: packages/client.customization/tables/CustomerExt.bos
tableextension CustomerLoyaltyExt extends Customer {
    loyaltyTier: Enum("Bronze", "Silver", "Gold", "Platinum") {
        default = "Bronze";
        caption = "Loyalty Tier";
    };
    loyaltyPoints: Integer {
        default = 0;
        caption = "Accrued Loyalty Points";
    };

    key(loyaltyTier, loyaltyPoints);

    onValidate(loyaltyPoints, rec, ctx) {
        if (rec.loyaltyPoints >= 10000) {
            rec.loyaltyTier = "Platinum";
        } else if (rec.loyaltyPoints >= 5000) {
            rec.loyaltyTier = "Gold";
        } else if (rec.loyaltyPoints >= 1000) {
            rec.loyaltyTier = "Silver";
        }
    }
}
```

### 5.2 The Event Subscriber Pattern (`[EventSubscriber]`)
Developers intercept core business flows cleanly without touching standard application source files:

```typescript
// File: packages/client.customization/codeunits/SalesDiscountHook.bos
codeunit SalesDiscountSubscriber {
    // Intercepts Sales Order insertion to apply automated loyalty discounts
    [EventSubscriber(Table::SalesHeader, "onBeforeInsert")]
    public applyLoyaltyDiscount(rec: SalesHeaderRecord, ctx: BosSessionContext) {
        // Fetch customer using First-Class Symbol
        const customer = Customer.get(rec.customerNo);
        if (!customer) return;

        // Apply discount based on custom loyaltyTier field from table extension!
        if (customer.loyaltyTier === "Platinum") {
            rec.discountPercentage = 15.0;
            ctx.audit.log(`Applied 15% Platinum discount to Order ${rec.orderNo}`);
        } else if (customer.loyaltyTier === "Gold") {
            rec.discountPercentage = 10.0;
            ctx.audit.log(`Applied 10% Gold discount to Order ${rec.orderNo}`);
        }
    }
}
```

---

## 6. Tier 4: Partner ISV Extensions (High-Compute Plugins)

### 6.1 WebAssembly (Wasm) High-Performance Plugin
For compute-intensive operations that must run safely without direct host access:

```
┌────────────────────────────────────────────────────────┐
│           BOS-Script Trigger (QuickJS)                 │
│  "const qr = ctx.wasm.call('qr.wasm', 'render', ...);" │
└───────────────────────────┬────────────────────────────┘
                            │ Host FFI
                            ▼
┌────────────────────────────────────────────────────────┐
│       Embedded Wasmtime Sandbox Engine (Rust)          │
│       - Max 16 MB Memory                               │
│       - Deterministic instruction counting             │
│       - Zero OS filesystem/network access               │
└────────────────────────────────────────────────────────┘
```

#### Step 1: Partner compiles Rust/Go code to `qr_engine.wasm`:
```rust
// Partner source in Rust:
#[no_mangle]
pub extern "C" fn render_qr(text_ptr: *const u8, text_len: usize) -> *const u8 {
    // Generates PNG bytes using qrcode crate in pure memory
}
```

#### Step 2: Consumed inside BOS-Script invoice trigger:
```typescript
// File: packages/partner.einvoicing/triggers/InvoiceQrHook.bos
codeunit InvoiceQrSubscriber {
    [EventSubscriber(Table::SalesHeader, "onAfterInsert")]
    public generateInvoiceQr(rec: SalesHeaderRecord, ctx: BosSessionContext) {
        const qrPayload = `UPI://PAY?PA=FINANCE@ACME&AM=${rec.totalAmount}&TR=${rec.orderNo}`;
        
        // Execute sandboxed Wasm plugin with instruction fuel metering
        const taxVal = ctx.wasm.invoke("tax_calc", "calc_tax", rec.subtotal, rec.taxRate);
        rec.taxAmount = taxVal;
        SalesHeader.update(rec.orderNo, { taxAmount: taxVal });
    }
}
```

---

### 6.2 Out-of-Process Sidecar Microservices
When an integration requires complex operating system libraries, AI models, or third-party SDKs (e.g. AWS S3, Stripe, OpenAI, SAP RFC connectors):

```
┌────────────────────────┐                   ┌───────────────────────────────┐
│   BOS Engine (Rust)    │  Loopback HTTP    │   Partner Sidecar Service     │
│   QuickJS Sandbox      ├──────────────────►│   (Local process / container) │
│   (Max 8 MB RAM)       │  IPC Bridge       │   - AI Invoice OCR / Stripe   │
└────────────────────────┘                   └───────────────────────────────┘
```

#### Step 1: Register sidecar service via REST API:
```http
POST /api/sidecars HTTP/1.1
Content-Type: application/json

{
  "name": "ai_ocr",
  "endpoint_url": "http://127.0.0.1:5005",
  "command": "python",
  "args": ["ocr_service.py"],
  "description": "Invoice OCR deep learning model"
}
```

#### Step 2: Consumed cleanly in BOS-Script:
```typescript
codeunit InvoiceOcrProcessor {
    public extractVendorInvoice(pdfBlob: any, ctx: BosSessionContext): any {
        // High-speed loopback IPC call to isolated sidecar companion
        const extractedData = ctx.sidecar.call("ai_ocr", "/extract", {
            documentPayload: pdfBlob
        });
        
        return extractedData;
    }
}
```

---

## 7. Package Deployment & Lifecycle Validation (`bosc package`)

All partner and tenant customizations are packaged into cryptographically signed archives (`.bospkg`):

```bash
# 1. Compile, lint, and package extension
bosc package --path ./packages/partner.logistics --output ./dist/partner-logistics-1.0.0.bospkg

# 2. Deploy to active tenant database
bosc deploy --package ./dist/partner-logistics-1.0.0.bospkg --db "postgresql://localhost:5432/bos_sandbox"
```

### Safety Guards During Deployment:
1. **Dependency Verification:** Ensures required base tables exist before mounting.
2. **Deterministic Schema DDL:** Computes non-destructive schema deltas (`ADD COLUMN`, `CREATE INDEX`). Destructive drops are rejected.
3. **Event Registration:** Automatically registers all `[EventSubscriber]` methods in the in-memory runtime event dispatcher.
4. **Instant Database Backup/Portability:** All extension definitions, event subscriber mappings, and table metadata are saved in PostgreSQL table `_bos_meta_objects`, ensuring 100% portability during backup and restore (`pg_dump` / `pg_restore`).
