# Meta-Object Specification: `codeunit`, `interface`, `enum` & `enumextension`

> **Status:** Proposed Specification for Review & Freezing  
> **Document:** `docs/01_meta_object_codeunit_interface_enum_specification.md`  
> **References:** Business Central AL Codeunit architecture, Polymorphic Strategy Pattern, WASM Sandbox Execution, Transactional Boundaries.  

---

## 1. Why `codeunit`, `interface`, and `enum`?

In enterprise ERP platforms, business logic must be:
1. **Decoupled from UI and Schemas:** Logic belongs in dedicated, reusable service objects, not buried in form click handlers or database triggers.
2. **Strictly Transactional:** Financial postings across dozens of ledger tables must commit atomically or roll back completely on failure.
3. **Polymorphic & Non-Invasively Extensible:** Adding a new payment provider, tax calculation engine, or carrier integration should never require modifying base code or writing ugly `switch/case` statements.

---

## 2. Meta-Object 1: `codeunit` (Business Logic Execution Unit)

A **`codeunit`** is a compiled, encapsulated unit of procedural business logic, transactional posting routines, or background worker tasks.

### 2.1 Compilation & Execution Model
- A codeunit method runs as JavaScript in the QuickJS host (`bosc-runtime`), after type erasure. It does not compile to Wasm.
- The host allows 8 MB of QuickJS memory, a 512 KB stack, and a fuel budget. The runtime is reused from a thread-local pool.
- Database work goes through the model API (`Customer.get`, `createMany`, `modify`) or `ctx.db`. A method's `return` value is what `ctx.call` receives.
- Wasm is a separate host (`ctx.wasm.invoke`) for numeric exports loaded by `wasmi`. It is not the codeunit compiler.

### 2.2 Execution Profiles

| Profile / Modifier | Purpose | Behavior |
| :--- | :--- | :--- |
| **`tableNo = Table.Entity`** | Record-Bound Codeunit | Binds directly to an entity type; automatically receives `this.rec` and `this.xRec`. |
| **`@transactional` (Default)**| Atomic Transaction | Runs in an isolated DB transaction; any unhandled exception or `error()` triggers instant rollback. |
| **`@exposed`** | API & RPC Exposure | Auto-generates typed JSON-RPC endpoints and OpenAPI documentation. |
| **`@job(queue, retry)`** | Background Worker | Dispatches execution to background task worker queues with auto-retry on transient lock failures. |
| **`@test`** | Test Service | Executes in an auto-rollback transaction for zero-pollution unit testing. |

### 2.3 Syntax Example: `SalesPostService.bos`

```typescript
codeunit SalesPostService {
    caption = "Sales Document Posting Routine";
    tableNo = Table.SalesHeader;

    // Public entry point
    @exposed
    public static post(orderId: UUID): PostedInvoiceResult {
        const header = SalesHeader.get(orderId);
        return this.run(header);
    }

    // Record-bound execution entry point
    public run(header: SalesHeader): PostedInvoiceResult {
        // Step 1: Invariant Verification
        this.checkOrder(header);

        // Step 2: Posting Routine
        const invoiceNo = this.postLines(header);

        // Step 3: Ledger Postings
        this.postCustomerLedger(header, invoiceNo);
        this.postGeneralLedger(header, invoiceNo);

        // Step 4: Status Update & Outbox Notification
        header.status = "Invoiced";
        header.save();

        DomainEvents.emit(new OrderInvoicedEvent({
            orderId: header.id,
            invoiceNo: invoiceNo,
            customerId: header.customerId,
            totalAmount: header.totalAmount
        }));

        return { invoiceNo };
    }

    private checkOrder(header: SalesHeader) {
        header.testField("status", "Approved");
        header.testField("customerId");
        header.calcFields("totalAmount", "totalQuantity");

        if (header.totalAmount <= 0) {
            error("Cannot post sales order with total amount <= 0.");
        }
    }

    private postCustomerLedger(header: SalesHeader, invoiceNo: String) {
        // Post write-once immutable Customer Ledger Entry
        CustomerLedgerEntry.insert({
            postingDate: header.orderDate,
            documentType: "Invoice",
            documentNo: invoiceNo,
            customerId: header.customerId,
            amount: header.totalAmount,
            remainingAmount: header.totalAmount,
            open: true
        });
    }

    private postGeneralLedger(header: SalesHeader, invoiceNo: String) {
        // Double-entry General Ledger balancing
        const postingGroup = CustomerPostingGroup.get(header.customerPostingGroup);
        
        // Debit: Accounts Receivable
        GeneralLedgerEntry.insert({
            postingDate: header.orderDate,
            documentNo: invoiceNo,
            accountNo: postingGroup.receivablesAccount,
            amount: header.totalAmount,
            description: `Invoice for Customer ${header.customerId}`
        });

        // Credit: Sales Revenue
        GeneralLedgerEntry.insert({
            postingDate: header.orderDate,
            documentNo: invoiceNo,
            accountNo: postingGroup.salesAccount,
            amount: -header.totalAmount,
            description: `Revenue for Order ${header.no}`
        });
    }
}
```

---

## 3. Meta-Object 2: `interface` (Contract Definition)

Interfaces define pure polymorphic contracts for business logic without implementation details.

```typescript
interface IPaymentProcessor {
    caption = "Payment Gateway Interface";
    processPayment(order: SalesHeader, amount: Decimal(18, 2)): PaymentResult;
    refundPayment(transactionRef: String, amount: Decimal(18, 2)): RefundResult;
}
```

---

## 4. Meta-Object 3: `enum` & `enumextension` with First-Class Strategy Pattern

BOS Script elevates enums from simple scalar constants to **First-Class Strategy Pattern Dispatches** (modeled after modern Business Central AL interface enums).

### 4.1 Base Extensible Enum Implementing Interface
```typescript
enum PaymentMethod implements IPaymentProcessor {
    extensible = true;
    caption = "Accepted Payment Methods";

    Cash = Codeunit.CashPaymentProcessor;
    BankTransfer = Codeunit.BankTransferProcessor;
    CreditCard = Codeunit.StripePaymentProcessor;
}
```

### 4.2 Non-Invasive Extension (`enumextension`)
A third-party localization or payment app introduces a new provider without modifying base code:

```typescript
enumextension PayPalPaymentMethodExt extends PaymentMethod {
    PayPal = Codeunit.PayPalPaymentProcessor;
    ApplePay = Codeunit.ApplePayPaymentProcessor;
}
```

### 4.3 Zero-Boilerplate Polymorphic Dispatch in Code
In traditional systems, developers write brittle switch statements:
```typescript
// Traditional Brittle Approach (Bad):
switch (order.paymentMethod) {
    case "Cash": new CashProcessor().process(); break;
    case "CreditCard": new StripeProcessor().process(); break;
    // Missing PayPal! Breaks when new extensions are installed!
}
```

In BOS Script, the compiler automatically enables direct polymorphic invocation:
```typescript
// BOS Script Polymorphic Dispatch (World-Class):
codeunit CheckoutService {
    public static checkout(order: SalesHeader) {
        // Dynamically instantiates the correct bound codeunit at runtime!
        const processor: IPaymentProcessor = order.paymentMethod;
        const result = processor.processPayment(order, order.totalAmount);
        
        if (!result.success) {
            error(`Payment failed: ${result.errorMessage}`);
        }
    }
}
```

---

## 5. Event Publishing & Subscribing Architecture

Codeunits serve as the backbone for non-invasive event-driven customization across packages:

### 5.1 Publishing Integration Events
```typescript
codeunit SalesPostService {
    // Declarative event hook point
    [IntegrationEvent]
    public static onBeforePost(header: SalesHeader, args: CancelEventArgs) {}

    [IntegrationEvent]
    public static onAfterPost(header: SalesHeader, invoiceNo: String) {}
}
```

### 5.2 Subscribing to Integration Events (in Third-Party Apps)
```typescript
codeunit ComplianceAuditExtension {
    [EventSubscriber(SalesPostService, "onBeforePost", priority = 50)]
    public static verifyTaxId(header: SalesHeader, args: CancelEventArgs) {
        if (!header.vatRegistrationNo) {
            args.cancel("VAT Registration Number is required before posting.");
        }
    }

    [EventSubscriber(SalesPostService, "onAfterPost")]
    public static notifyWarehouse(header: SalesHeader, invoiceNo: String) {
        WarehouseQueue.enqueueFulfillment(invoiceNo);
    }
}
```

---

## 6. Built-In Testing DSL (`@test` Codeunits)

Testing in BOS Script requires no external runners or mock frameworks. The Rust compiler compiles test codeunits with **transactional isolation and auto-rollback**:

```typescript
@test
codeunit SalesPostTests {
    caption = "Sales Posting Engine Verification";

    test "Posting Sales Order creates balanced GL Entries" {
        // Arrange
        const customer = TestFixture.create(Customer, { creditLimit: 50_000 });
        const order = TestFixture.create(SalesHeader, {
            customerId: customer.id,
            status: "Approved"
        });
        TestFixture.create(SalesLine, {
            documentId: order.id,
            quantity: 2,
            unitPrice: 1_000
        });

        // Act
        const result = SalesPostService.post(order.id);

        // Assert
        Assert.isNotNull(result.invoiceNo);

        const glEntries = query GeneralLedgerEntry where { documentNo == result.invoiceNo };
        const totalDebit = glEntries.filter(e => e.amount > 0).reduce((sum, e) => sum + e.amount, 0);
        const totalCredit = glEntries.filter(e => e.amount < 0).reduce((sum, e) => sum + Math.abs(e.amount), 0);

        Assert.equals(totalDebit, totalCredit); // Invariant: Debits == Credits!
        // End of test: DB transaction automatically rolled back with zero pollution!
    }
}
```
