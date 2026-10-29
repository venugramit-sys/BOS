# Volume 2: End-User & Operator Manuals
## 05. Notifications & Audit Trails

BOS provides unified real-time messaging, asynchronous task alerts, and tamper-evident cryptographic audit logs for complete visibility into enterprise operations.

---

## 🔔 The Notification Center

Click the **Bell** icon in the upper right header to open the slide-out **Notification Center Drawer**:

```
┌────────────────────────────────────────────────────────┐
│ Notifications                                 [Clear] ✖│
├────────────────────────────────────────────────────────┤
│ 🔵 Approval Request                       10 mins ago  │
│ Purchase Order PO-2026-0089 requires your approval.   │
│ [ View Document ]                                      │
├────────────────────────────────────────────────────────┤
│ 🟢 Background Report Completed            25 mins ago  │
│ Monthly VAT / GST Reconciliation report is ready.      │
│ [ Download PDF ]                                       │
├────────────────────────────────────────────────────────┤
│ 🟡 Inventory Alert                         1 hour ago  │
│ Stock for Item ITM-1001 is below minimum reorder point.│
└────────────────────────────────────────────────────────┘
```

### Notification Types:
- **Actionable Requests**: Approvals, signature requests, or validation alerts. Clicking the button opens the target record immediately.
- **Asynchronous Task Alerts**: Notifications when long-running batch jobs, reports, or data imports finish.
- **System Warnings**: License warnings, scheduled maintenance notices, or storage threshold alerts.

---

## 🍞 Transient Toasts

When you perform standard actions, floating toast messages appear in the bottom right corner of the screen:
- **Success (Green)**: "Customer CUST-00101 saved successfully."
- **Info (Blue)**: "Report queued in background worker."
- **Warning (Amber)**: "Credit limit threshold reached."
- **Error (Red)**: "Validation failed: Postcode format invalid." (Clicking the toast focuses the invalid input field).

---

## 📜 Reviewing Audit Trails & Cryptographic Chains

For compliance with statutory accounting standards and forensic security audits, BOS tracks every critical business mutation:

### Viewing the Audit History of a Record
1. Open the Card or Document page for any entity.
2. Click **History** in the right-side Contextual Pane.
3. The audit log displays:
   - **Timestamp (UTC)**
   - **User ID & Name**
   - **Action**: `INSERT`, `MODIFY`, or `DELETE`
   - **Field Diffs**: Exact before-and-after values
   - **Cryptographic Status**: `✓ Verified (Chain Intact)`

### Cryptographic Hash Chain Guarantee
Each audit row contains a SHA-256 hash calculated over the current payload combined with the preceding row's hash. If a rogue database administrator attempts to manually edit historical ledger entries in PostgreSQL, the hash chain breaks, immediately flagging the record during audits.
