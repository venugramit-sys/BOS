# Volume 5: Integration & API Manuals
## 03. WebAssembly Plugins & Companion Sidecars

When business operations require specialized algorithms, high-throughput computation, or integration with external binaries, BOS provides **`wasmi 2.0` WebAssembly Sandboxing** and **Loopback IPC Sidecars**.

---

## 🧩 WebAssembly Plugins (`wasmi 2.0`)

WebAssembly plugins run within the BOS process with deterministic execution limits, memory protection, and zero network/filesystem attack surface.

### Common Use Cases:
- Complex tax engine calculation routines (e.g., European VAT or Indian GST multi-slab calculations).
- PDF document and barcode/QR-code vector rendering.
- Cryptographic signature generation and hash computations.

### Authoring a Wasm Module (Rust Example):
```rust
// In your Wasm crate
#[no_mangle]
pub extern "C" fn calculate_gst(amount: f64, rate: f64) -> f64 {
    amount * (rate / 100.0)
}
```
Compile to WebAssembly:
```bash
cargo build --target wasm32-unknown-unknown --release
```

### Invoking Wasm from BOS-Script:
```typescript
// Inside tablelogic or codeunit
const wasmResult = ctx.wasm.invoke("gst_calculator.wasm", "calculate_gst", [
    this.taxable_amount,
    18.0
]);
this.tax_amount = wasmResult.toString();
```

---

## 🚗 Loopback IPC Companion Sidecars

For scenarios requiring communication with legacy enterprise software, proprietary vendor DLLs, or specialized OS hardware (such as scales, barcode scanners, and fiscal receipt printers), BOS supports **Loopback IPC Sidecars**:

```
┌────────────────────────────────────────────────────────┐
│ BOS Backend Engine (Rust Axum Process)                 │
└──────────────────────────┬─────────────────────────────┘
                           │ Loopback HTTP / mTLS (localhost only)
┌──────────────────────────▼─────────────────────────────┐
│ Sidecar Microservice (Python, Go, Node, or C# Binary)  │
│ Listening on: 127.0.0.1:8081                           │
└────────────────────────────────────────────────────────┘
```

### Configuring Sidecars in `bos-node.toml`:
```toml
[[sidecars]]
id = "fiscal-printer-gateway"
url = "http://127.0.0.1:8081"
timeout_ms = 5000
health_endpoint = "/health"
```

### Calling Sidecars in Business Logic:
```typescript
const response = ctx.sidecar.call("fiscal-printer-gateway", "/print-receipt", {
    orderNo: this.order_no,
    lines: this.lines,
    total: this.total_amount
});
```
Sidecars only accept connections originating from `127.0.0.1` and run under dedicated system permissions.
