# Volume 4: Developer Manuals
## 05. Page Kit & Custom UI Development

BOS pages combine declarative DSL layout definitions with the high-performance `@bos/page-kit` dynamic browser runner, powered by modern Svelte 5 runes.

---

## 🎨 Page Meta-Object Syntax

Pages define the layout, data source bindings, actions, and subforms:

```bos
page VendorCard for Vendor {
    type: Card;
    caption: "Vendor Details";
    sourceTable: Vendor;

    layout {
        area(Content) {
            group("General Information") {
                field(vendor_no; "No.") {
                    editable: false;
                }
                field(name; "Vendor Name") {
                    importance: Promoted;
                }
                field(currency_code; "Currency") {}
                field(blocked; "Blocked") {}
            }

            group("Financials") {
                field(balance; "Current Balance") {
                    importance: Promoted;
                }
                field(credit_limit; "Credit Limit") {}
                field(payment_terms; "Payment Terms") {}
            }
        }

        area(FactBoxes) {
            part(VendorHistory; "VendorHistoryFactBox") {
                subFormLink: "vendor_no = field(vendor_no)";
            }
        }
    }

    actions {
        action(LedgerEntries) {
            caption: "Ledger Entries";
            image: "BookOpen";
            runPage: "VendorLedgerEntries";
            runPageLink: "vendor_no = field(vendor_no)";
        }
        action(PayVendor) {
            caption: "Make Payment";
            image: "CreditCard";
            promoted: true;
            runCodeunit: "VendorPaymentCodeunit";
        }
    }
}
```

---

## ⚡ Dynamic Page Kit Engine (`@bos/page-kit`)

The Svelte 5 runtime interprets compiled Page IR JSON envelopes without recompiling the web application:

1. **Reactive State via Runes**: Form values are bound using `$state` primitives.
2. **Computed FlowFields**: Dynamically retrieved and cached in `$derived` runes.
3. **Automatic Dirty Tracking**: Every keystroke updates a reactive modified dirty set.
4. **Accessible Input Components**: Built on top of Tailwind CSS and Lucide icons with complete WAI-ARIA keyboard navigation.

---

## 💻 Custom Component Embedding

For custom visualization requirements (e.g., custom charting, interactive Gantt schedules, geographic maps):
1. Create a custom Svelte 5 component under `ui/src/lib/components/custom/`.
2. Reference the component in your page definition via the `customControl` directive:
   ```bos
   group("Production Schedule") {
       customControl("GanttChart") {
           props: '{"dataSource": "ProductionOrders"}';
       }
   }
   ```
3. `@bos/page-kit` resolves and mounts the component into the page slot dynamically.
