# Volume 2: End-User & Operator Manuals
## 02. Working with Data Records & Lists

List pages are the central tool for viewing, searching, sorting, and analyzing operational business records (such as Customers, Vendors, Invoices, Items, and General Ledger Entries).

---

## 📋 List Page Features

```
┌────────────────────────────────────────────────────────────────────────┐
│ Customers                                           [+ New] [Export ▾] │
├────────────────────────────────────────────────────────────────────────┤
│ 🔍 Search records...     | Filter: [ Status = Active ▾ ] [ Balance > 0]│
├──────────────┬──────────────────────┬───────────────┬──────────────────┤
│ Customer No. │ Name                 │ Balance (USD) │ Status           │
├──────────────┼──────────────────────┼───────────────┼──────────────────┤
│ CUST-00101   │ Acme Industrial Corp │ 14,250.00     │ Active           │
│ CUST-00102   │ Global Logistics Ltd │      0.00     │ Active           │
│ CUST-00103   │ Apex Retailers Inc   │ 98,400.00     │ Overdue          │
├──────────────┴──────────────────────┴───────────────┴──────────────────┤
│ Showing 1-3 of 1,420 records                    [< Prev] [1] [2] [Next >]│
└────────────────────────────────────────────────────────────────────────┘
```

---

## 🔍 Finding Records: Search & Advanced Filters

### 1. Instant Text Search
Type any search term into the search bar at the top of the table. The search scans across all indexed text columns (such as `customer_no`, `name`, `tax_id`, and `city`) simultaneously.

### 2. Column-Level Sorting
- Click any column header to sort in **Ascending** order.
- Click the column header a second time to sort in **Descending** order.
- A small arrow indicator (`▲` / `▼`) denotes the active sort order.

### 3. Filter Pane
Click the **Filter** icon next to the search bar to apply multi-field logical filters:
- **String Filters**: `Contains`, `Starts with`, `Exact match`, `Does not equal`
- **Numeric & Currency Filters**: `=`, `>`, `<`, `>=`, `<=`, `Between (x .. y)`
- **Date Filters**: `Today`, `This Month`, `Past 30 Days`, `Custom Range`
- **Status Filters**: Multi-select dropdown checkboxes.

---

## ➕ Creating New Records

1. Click the **`+ New`** button in the upper right corner (or press `Alt + N`).
2. A new Card / Document Form page will open automatically.
3. If an automated **Number Series** is configured for the table, the primary key field (e.g., `Customer No.`) is auto-generated upon creation.

---

## ✏️ Selecting & Opening Records

- **Open Record**: Double-click any row (or highlight it and press `Enter`) to open the full Card view.
- **Multi-Select**: Hold `Ctrl` or `Shift` while clicking row checkboxes to select multiple records for bulk actions (e.g., Export, Delete, or Batch Status Update).

---

## 📤 Exporting Data

Click the **Export** button in the toolbar to export the active filtered list:
- **Export to CSV**: Raw comma-delimited format suitable for spreadsheets and data science tools.
- **Export to JSON**: Formatted JSON array representing the current table view and FlowFields.
