# BC journal and list keyboard editing

Scope: clerk-facing cell editing on Business Central lists and journals (prefer 2024–2026 Microsoft Learn). NAV Windows client behavior is marked NAV-only. Excel workbook features are omitted. No shortcut is listed unless a source states it.

Primary pages used:

- [Keyboard Shortcuts](https://learn.microsoft.com/en-us/dynamics365/business-central/keyboard-shortcuts)
- [Enter data](https://learn.microsoft.com/en-us/dynamics365/business-central/ui-enter-data)
- [Entering dates and times](https://learn.microsoft.com/en-us/dynamics365/business-central/ui-enter-date-ranges) (page title on Learn is “Entering dates and times”; related link text is “Work with calendar dates and times”)
- [Working with general journals](https://learn.microsoft.com/en-us/dynamics365/business-central/ui-work-general-journals)
- [Warnings and error messages](https://learn.microsoft.com/en-us/dynamics365/business-central/product-error-messages)
- [NAV 2018 Windows client keyboard shortcuts](https://learn.microsoft.com/en-us/previous-versions/dynamicsnav-2018/keyboard-shortcuts-windows-client) (retired)

## What does a BC journal line grid do for cell focus, edit mode, and leaving a field (Enter, Tab, Shift+Tab, arrows, F2)?

### Takeaway

A journal or list is a keyboard grid. Focus selects the whole cell value; F2 toggles that selection against a cursor at the end of the value. Tab and Shift+Tab move to the next or previous column in an editable list. Up and Down move one row in the same column. In an editable list, Left and Right move inside the current value and only change columns when the cursor is already at the start or end. Enter is documented twice: Quick Entry confirms the cell and jumps to the next included field, and a separate list shortcut opens a card only when the row has one.

### Cited Findings

- “Many page types in Business Central contain grids that consist of rows and columns. Examples include list pages and Lines parts in documents. Movement from one cell to another in a grid is fully keyboard-enabled.” — [Keyboard Shortcuts](https://learn.microsoft.com/en-us/dynamics365/business-central/keyboard-shortcuts)
- Focus: “To modify the value of a field, you must first set focus to that field.” Tab “selects the entire value.” A left-click “will only select the entire field value if the field is in a list.” The client “typically favors selecting the entire field value” so typing replaces it. — [Enter data](https://learn.microsoft.com/en-us/dynamics365/business-central/ui-enter-data)
- While the whole value is selected: typing replaces it; Delete or Backspace clears it; Alt+Down Arrow activates a picker if the field has one. — [Enter data](https://learn.microsoft.com/en-us/dynamics365/business-central/ui-enter-data)
- F2 “Switch between selecting the entire field value and placing the cursor at the end of the field value.” — [Keyboard Shortcuts](https://learn.microsoft.com/en-us/dynamics365/business-central/keyboard-shortcuts). Same sentence on [Enter data](https://learn.microsoft.com/en-us/dynamics365/business-central/ui-enter-data): F2 toggles select-all versus the cursor after the value so you can append.
- With the cursor at the end: typing appends. Home, End, Left Arrow, and Right Arrow move inside the value. “If you're editing a field in a list, selecting the Left Arrow key again when the cursor is at the beginning of the value will set the focus to the previous field. Similarly, selecting the Right Arrow key again when the cursor is at the end of the value will set the focus to the next field.” — [Enter data](https://learn.microsoft.com/en-us/dynamics365/business-central/ui-enter-data)
- Editable-list arrows, separate from the in-value rule above: Right Arrow “move to the right within the current field”; Left Arrow “move to the left within the current field.” In a view-only list the same keys move one column. Down Arrow moves “one row down within the same column”; Up Arrow moves “one row up within the same column.” — [Keyboard Shortcuts](https://learn.microsoft.com/en-us/dynamics365/business-central/keyboard-shortcuts)
- Tab in an editable list: “move to the field that is one column to the right within the same row.” Shift+Tab: “one column to the left within the same row.” Both are “Not applicable” in tile view. Page-level Tab / Shift+Tab also “Move focus to the next/previous control or element,” including a field or list heading. — [Keyboard Shortcuts](https://learn.microsoft.com/en-us/dynamics365/business-central/keyboard-shortcuts)
- Quick Entry Enter, on cards, documents, and list pages (not tile view): Enter “Confirm the value in the current field, and go to the next Quick Entry field.” Shift+Enter “Confirm the value in the current field, and go to the previous Quick Entry field.” Ctrl+Shift+Enter “Confirm the value in the current column, and go to the next Quick Entry field outside the list” (embedded lists such as document lines). — [Keyboard Shortcuts](https://learn.microsoft.com/en-us/dynamics365/business-central/keyboard-shortcuts)
- Conflicting list Enter: under “Navigate between rows and columns,” Enter “Open the record that is associated with the field,” and “This keyboard shortcut is relevant only if a card page is associated with the record.” — [Keyboard Shortcuts](https://learn.microsoft.com/en-us/dynamics365/business-central/keyboard-shortcuts)
- General Enter (not list-specific): “Enable or access the element or control that is in focus.” Esc “Close the current page or dropdown list.” — [Keyboard Shortcuts](https://learn.microsoft.com/en-us/dynamics365/business-central/keyboard-shortcuts)
- Ctrl+Enter in a list: “Move focus to the next element outside the list.” The same keys in the filter pane “Move focus from the filter pane back to the list.” On a card or document, Ctrl+Enter “Save and close the page.” — [Keyboard Shortcuts](https://learn.microsoft.com/en-us/dynamics365/business-central/keyboard-shortcuts)
- Other grid moves: Home / End go to the first / last field in the row. Ctrl+Home / Ctrl+End select the first / last row and “The focus remains in the same column.” Page Up / Page Down scroll the row set. — [Keyboard Shortcuts](https://learn.microsoft.com/en-us/dynamics365/business-central/keyboard-shortcuts)
- Calendar exception: Ctrl+Home opens and closes the date picker, but “This keyboard shortcut doesn't work if the date field is in a grid. In that case, Ctrl+Home jumps to the first row of the grid.” — [Keyboard Shortcuts](https://learn.microsoft.com/en-us/dynamics365/business-central/keyboard-shortcuts)
- Value is not checked while you type: “After you specify a value, Business Central will only check that it's valid after you click outside the field or set focus to another element, such as the next field.” — [Enter data](https://learn.microsoft.com/en-us/dynamics365/business-central/ui-enter-data)
- General Journal is a worksheet with `DelayedInsert = True` and `AutoSplitKey = True` (`PageType = Worksheet`, source `Gen. Journal Line`). Item Journal is the same pattern. — [Page "General Journal"](https://learn.microsoft.com/en-us/dynamics365/business-central/application/base-application/page/microsoft.finance.generalledger.journal.general-journal); [Page "Item Journal"](https://learn.microsoft.com/en-us/dynamics365/business-central/application/base-application/page/microsoft.inventory.journal.item-journal)
- NAV-only, Windows client: Enter “Move to the next editable field.” F2 is only “Edit” (no select-all versus end-of-value wording). Tab “Move to the next field on non-line FastTabs.” Shift+Tab “Move to the previous field.” Arrows move to the next/previous field or character, or the field above/below in the same column, with no view-only versus editable split. — [NAV 2018 Windows client shortcuts](https://learn.microsoft.com/en-us/previous-versions/dynamicsnav-2018/keyboard-shortcuts-windows-client)

### Inferences

- Journal lines are the editable-list case: Tab/Shift+Tab change columns; Left/Right edit inside the cell until the cursor hits an edge. That reading follows the editable-list rows on the shortcut page plus the list-specific Left/Right sentences on Enter data. Microsoft never names “General Journal” in those tables.
- On a journal, which has no card page in the application metadata above, the “open the record” Enter is the branch the shortcut page says is relevant only when a card exists. Quick Entry Enter is the branch that matches line entry. The page does not say which Enter wins if both could apply.

### Gaps

- No current user article states a distinct “edit mode” you must enter before a journal cell accepts typing, beyond focus plus the Edit list icon (see the Quick Entry section).
- No sentence says what Enter does on a journal line when Quick Entry and “open the record” could both match.
- No sentence says whether Up/Down commit the cell before moving, other than the general rule that validity is checked when focus leaves the field.

## Which shortcuts are documented for lists and journals?

### Takeaway

Current Business Central documents Alt+Down for lookup, F8 for copy-above, F6/Shift+F6 for FastTabs, F7 statistics, Ctrl+F7 entries, Alt+F7 sort, F9 post, Ctrl+Enter leave-list or save-close, Alt+F9 post-and-new-document, and Ctrl+Delete to delete a journal line. F4 and Shift+F7 are documented for the NAV 2018 Windows client, not in the current Business Central shortcut article. Ctrl+Shift+F7 is not on either Microsoft Learn shortcut page.

### Cited Findings

Keys the question named:

- Alt+Down arrow: “Open a dropdown list or look up a value for a field.” — [Keyboard Shortcuts](https://learn.microsoft.com/en-us/dynamics365/business-central/keyboard-shortcuts). Enter data repeats: activate a picker with Alt+Down Arrow. — [Enter data](https://learn.microsoft.com/en-us/dynamics365/business-central/ui-enter-data)
- F8, lists only, editable lists only: “Copy the field that is one row up within the same column, and paste it into the current row.” Then Tab “quickly fill in the fields for line items that you want to have the same values as the previous row.” Tile view: not applicable. — [Keyboard Shortcuts](https://learn.microsoft.com/en-us/dynamics365/business-central/keyboard-shortcuts). Enter data repeats F8 for “the field in the same column of the row above.” — [Enter data](https://learn.microsoft.com/en-us/dynamics365/business-central/ui-enter-data)
- F6, cards and documents: “Move to the next FastTab or part (subpage).” Shift+F6: previous FastTab or part. Alt+F6: “Collapse or expand the current FastTab or part (subpage).” — [Keyboard Shortcuts](https://learn.microsoft.com/en-us/dynamics365/business-central/keyboard-shortcuts)
- F7, lists and also cards/documents: “Open statistics.” — [Keyboard Shortcuts](https://learn.microsoft.com/en-us/dynamics365/business-central/keyboard-shortcuts)
- Ctrl+F7, lists and cards/documents: “Open ledger entries, log entries, cost entries, and other types of entries.” The intro adds that in the generic product “Ctrl+F7 shows the ledger entries for a record when you open the record in a card,” and “the tooltip for an action shows the keyboard shortcut” because shortcuts “might differ in your solution.” — [Keyboard Shortcuts](https://learn.microsoft.com/en-us/dynamics365/business-central/keyboard-shortcuts)
- Alt+F7, list view only: “Sort the selected column in ascending or descending order.” Not applicable in tile view. — [Keyboard Shortcuts](https://learn.microsoft.com/en-us/dynamics365/business-central/keyboard-shortcuts)
- F9, lists and cards/documents: “Post, issue, register, or reverse a document.” Intro: “in the generic version of Business Central, the F9 key posts a document.” — [Keyboard Shortcuts](https://learn.microsoft.com/en-us/dynamics365/business-central/keyboard-shortcuts)
- Ctrl+Enter: three documented effects, by context. List: “Move focus to the next element outside the list.” Filter pane: “Move focus from the filter pane back to the list.” Card/document: “Save and close the page.” — [Keyboard Shortcuts](https://learn.microsoft.com/en-us/dynamics365/business-central/keyboard-shortcuts)
- Alt+F9, cards and documents: “Create a new document, and post it.” Intro, next to Alt+Shift+N: “The effect of Alt+F9 is similar. It posts a document and creates a new one.” — [Keyboard Shortcuts](https://learn.microsoft.com/en-us/dynamics365/business-central/keyboard-shortcuts)
- Ctrl+Delete, cards and documents table: “Delete the line in documents, journals, and worksheets.” This is the shortcut text that names journals. — [Keyboard Shortcuts](https://learn.microsoft.com/en-us/dynamics365/business-central/keyboard-shortcuts)
- F4: not in the current Business Central shortcut article (the page was read in full). NAV-only, Windows client: F4 “Drop-down or look up to select.” — [NAV 2018 Windows client shortcuts](https://learn.microsoft.com/en-us/previous-versions/dynamicsnav-2018/keyboard-shortcuts-windows-client)
- Shift+F7: not in the current Business Central shortcut article. NAV-only: Shift+F7 “Open the related card.” — [NAV 2018 Windows client shortcuts](https://learn.microsoft.com/en-us/previous-versions/dynamicsnav-2018/keyboard-shortcuts-windows-client)
- Ctrl+Shift+F7: not listed on the current Business Central shortcut article and not listed on the NAV 2018 Windows client shortcut article. A non-Microsoft NAV 2009 cheat-sheet PDF maps classic-client “Clear all filters” to Ctrl+Shift+F7 and the RoleTailored client equivalent to Ctrl+Shift+A. That PDF is not a Microsoft Learn page. — [dynamicsnavfinancials.com NAV 2009 PDF](https://dynamicsnavfinancials.com/wp-content/uploads/2013/11/nav2009keyboardshortcuts.pdf). Current BC reset is different: Ctrl+Shift+Alt+F3 “Reset filters.” — [Keyboard Shortcuts](https://learn.microsoft.com/en-us/dynamics365/business-central/keyboard-shortcuts)

Other list, document, or journal shortcuts on the same BC article:

- Alt+N on an editable list: “Insert a new line into an editable list, such as the list on the G/L Budgets page.” Page-level Alt+N instead “Open a page where you can create a new record” (same effect as New or +). — [Keyboard Shortcuts](https://learn.microsoft.com/en-us/dynamics365/business-central/keyboard-shortcuts)
- Ctrl+Insert, cards and documents: “Insert a new line in documents.” — [Keyboard Shortcuts](https://learn.microsoft.com/en-us/dynamics365/business-central/keyboard-shortcuts)
- Shift+F9: “Post and print a document.” Ctrl+F9: “Release a document.” Shift+Ctrl+F9: “Post, print, and put away a warehouse receipt.” — [Keyboard Shortcuts](https://learn.microsoft.com/en-us/dynamics365/business-central/keyboard-shortcuts)
- Shift+F10: “Open a menu of options that are available for the selected row.” — [Keyboard Shortcuts](https://learn.microsoft.com/en-us/dynamics365/business-central/keyboard-shortcuts)
- Alt+D: “Open the dimension set entries.” Shift+Ctrl+I: “View serial and lot numbers that are assigned to the line item in the document or journal.” — [Keyboard Shortcuts](https://learn.microsoft.com/en-us/dynamics365/business-central/keyboard-shortcuts)
- Shift+F11: “Apply entries, get source documents, or get warehouse documents.” — [Keyboard Shortcuts](https://learn.microsoft.com/en-us/dynamics365/business-central/keyboard-shortcuts)
- Alt+G: “Open the Find Entries page… This keyboard shortcut also works for lists.” — [Keyboard Shortcuts](https://learn.microsoft.com/en-us/dynamics365/business-central/keyboard-shortcuts)
- Ctrl+Shift+F12: “Maximize the line items part on a document page.” Press again to restore. Enter data calls this “focus on line items.” — [Keyboard Shortcuts](https://learn.microsoft.com/en-us/dynamics365/business-central/keyboard-shortcuts); [Enter data](https://learn.microsoft.com/en-us/dynamics365/business-central/ui-enter-data)
- Filter and search on lists: F3 toggles search. Shift+F3 opens the filter pane on “Filter list by” or closes it without clearing filters. Ctrl+Shift+F3 does the same for “Filter total by.” Alt+F3 toggles “filtering to the selected value” and leaves focus on the cell. Shift+Alt+F3 adds a filter on the selected column and moves focus to that filter. Ctrl+Shift+Alt+F3 resets filters to the view default. — [Keyboard Shortcuts](https://learn.microsoft.com/en-us/dynamics365/business-central/keyboard-shortcuts)
- Ctrl+Alt+Down arrow: “Open a related record's card or list page filtered to the value in the lookup or assist edit field.” macOS form on the same row: Ctrl+Option+Down arrow. — [Keyboard Shortcuts](https://learn.microsoft.com/en-us/dynamics365/business-central/keyboard-shortcuts)
- Alt+Right arrow: “Look up additional information or underlying values for a field that contains the button,” used “when the usual dropdown button (Alt+Down arrow) in the same field is used for another purpose.” — [Keyboard Shortcuts](https://learn.microsoft.com/en-us/dynamics365/business-central/keyboard-shortcuts)
- Alt+Up arrow: show tooltip, validation error, Copilot suggestion, or why an agent picked the value; repeats toggle if more than one exists. Esc closes that popover and “return to the field input box.” — [Keyboard Shortcuts](https://learn.microsoft.com/en-us/dynamics365/business-central/keyboard-shortcuts)
- Copy/paste rows: Ctrl+C copies selected rows (or the whole field value if no inner text is selected). Ctrl+V pastes rows “only for editable lists.” Ctrl+A selects all rows. Shift+Up / Shift+Down extend the row selection. Ctrl+Up / Ctrl+Down move focus while keeping the current row selected. Ctrl+Spacebar toggles the focused row into the selection. — [Keyboard Shortcuts](https://learn.microsoft.com/en-us/dynamics365/business-central/keyboard-shortcuts)
- Date-field keys on the shortcut page: `t` “Enter the current date.” `w` “Enter the work date.” — [Keyboard Shortcuts](https://learn.microsoft.com/en-us/dynamics365/business-central/keyboard-shortcuts)
- Quick Entry trio again: Enter, Shift+Enter, Ctrl+Shift+Enter, as quoted in the focus section. — [Keyboard Shortcuts](https://learn.microsoft.com/en-us/dynamics365/business-central/keyboard-shortcuts)
- The article warns that business shortcuts “might differ in your solution” and that the action tooltip is the authority. — [Keyboard Shortcuts](https://learn.microsoft.com/en-us/dynamics365/business-central/keyboard-shortcuts)

NAV-only extras from the 2018 Windows client table (not repeated as current BC behavior):

- Shift+F1 “View error message.” Shift+F4 “Open a lookup window (from an ellipsis button).” Shift+F8 “Drop-down or look up to view.” Ctrl+F4 “Look up to the related list.” Ctrl+F11 “Reconcile or split lines.” Ctrl+Delete “Delete the selected line.” Ctrl+Insert “Insert new line.” Ctrl+Enter “Save and close window (equivalent to choosing the OK button).” Alt+Enter “Move to the field below without opening the drop-down menu.” F6 “Go to the next frame.” F8 “Copy the field above.” F9 “Post.” — [NAV 2018 Windows client shortcuts](https://learn.microsoft.com/en-us/previous-versions/dynamicsnav-2018/keyboard-shortcuts-windows-client)
- Developer trigger page, NAV 2018: OnLookup runs from the lookup button or F4. OnAssistEdit runs from the AssistEdit button or Shift+F4. OnDrillDown runs from the drill-down button or Shift+F8. — [Page and Action Triggers (NAV 2018)](https://learn.microsoft.com/en-us/previous-versions/dynamicsnav-2018-developer/Page-and-Action-Triggers)

### Inferences

- Ctrl+Delete is the only shortcut in the current article whose description contains the word “journals.” F8, F9, Alt+N, and the grid keys apply because journals are editable lists or document line parts, not because the article names a journal page.
- F4 in the web client is undocumented. Clerks are told to use Alt+Down. Treating F4 as a current BC shortcut would be an invention.

### Gaps

- The 2024 shortcut PDF is linked from Learn (`keyboard-shortcuts-2024.pdf`) but was not fully read. Search snippets show a function-key strip that includes an F4 key and the words “F8 Copy from cell above” and “Alt+Down arrow Open dropdown list.” They do not quote an action for F4. Do not treat F4 as documented for BC from that snippet.
- A third-party video transcript claims Shift+F7 opens cards and documents in Business Central. That is not a Microsoft page. Current Learn does not list Shift+F7.
- No Microsoft page found that assigns Ctrl+Shift+F7 in Business Central.

## How do Quick Entry, the new-row star, the pencil, delayed insert, and copy-above work?

### Takeaway

Quick Entry is Enter (and Shift+Enter) through fields marked included, including across rows, while Tab still walks every editable field. Copy-above is F8. New lines are Alt+N on an editable list and Ctrl+Insert on documents; Ctrl+Delete removes a journal line. DelayedInsert on the General Journal page inserts the row when the user leaves the record. A red asterisk is a mandatory-field reminder, not a new-row marker. Current user docs do not describe a row-selector star or an in-grid pencil.

### Cited Findings

Quick Entry:

- “Quick Entry works on fields (like on card pages) and in lists (rows and columns).” Tab “always goes sequentially to the next field.” Enter goes “through only those fields that you're interested in” and “skips non-editable fields and fields that you typically don't fill in.” — [Enter data](https://learn.microsoft.com/en-us/dynamics365/business-central/ui-enter-data)
- Included fields are on the Enter path; excluded fields are not. Enter confirms and goes to the next field; Shift+Enter goes to the previous. — [Enter data](https://learn.microsoft.com/en-us/dynamics365/business-central/ui-enter-data)
- “It works across columns and rows.” Actions stay reachable with Tab and Shift+Tab. A collapsed FastTab expands if the next Quick Entry field is inside it. Mandatory fields are not automatically on the path, so they should be included. “By default, most fields are automatically included.” — [Enter data](https://learn.microsoft.com/en-us/dynamics365/business-central/ui-enter-data)
- Personalize: in a list, select the column heading, then “Include in Quick Entry” or “Exclude from Quick Entry.” — [Enter data](https://learn.microsoft.com/en-us/dynamics365/business-central/ui-enter-data). Personalize docs: “Skip over a field when pressing Enter” by choosing Exclude from Quick Entry on the field or column heading. If that command is missing, the field is already skipped. — [Personalize your workspace](https://learn.microsoft.com/en-us/dynamics365/business-central/ui-personalization-user)
- Shortcut table matches: Enter next, Shift+Enter previous, Ctrl+Shift+Enter out of an embedded list. “For list pages, you can't use these shortcuts when the list is in the tile view.” — [Keyboard Shortcuts](https://learn.microsoft.com/en-us/dynamics365/business-central/keyboard-shortcuts)

New line, not a star:

- Alt+N “Insert a new line into an editable list, such as the list on the G/L Budgets page.” — [Keyboard Shortcuts](https://learn.microsoft.com/en-us/dynamics365/business-central/keyboard-shortcuts)
- Ctrl+Insert “Insert a new line in documents.” Ctrl+Delete “Delete the line in documents, journals, and worksheets.” — [Keyboard Shortcuts](https://learn.microsoft.com/en-us/dynamics365/business-central/keyboard-shortcuts)
- Red asterisk is unrelated to a new row: “certain fields are marked with a red asterisk. The red asterisk means the field must be filled in to complete a certain process.” You are not required to fill it “before you continue to other fields or close the page.” It “only serves as a reminder that you'll be blocked from completing a certain process.” — [Enter data](https://learn.microsoft.com/en-us/dynamics365/business-central/ui-enter-data)

Pencil / Edit list (action-bar icon, not a row glyph):

- If a list page has `Editable = true`, “the Edit list icon is available in the action bar and by clicking the icon, the user can edit values across rows.” Lists opened from the Role Center “are always opened in the read-only mode, regardless of the Editable property.” — [Designing List Pages](https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/devenv-designing-list-pages)
- Card pages: system-action icons “provide users the ability to edit the record, create a new record, and delete the current record.” They “are only active if the Editable property is set to true” and “appear on all pages; you cannot remove them.” — [Designing card pages](https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/devenv-designing-card-pages)
- Journals can show more columns: “By default, the general journal only shows a limited number of fields on the journal line.” Show More Columns reveals fields such as Account Type. “When you show fewer columns, the same posting date is used for all lines.” — [Working with general journals](https://learn.microsoft.com/en-us/dynamics365/business-central/ui-work-general-journals)

Delayed insert:

- `DelayedInsert`: “whether a user must leave a record before it is inserted into the database. By default, new records are inserted when the user leaves the control that shows the primary key.” True means “the record is inserted when the user leaves the record”; false means insert “when the user leaves the control that shows the primary key.” Default is false. — [DelayedInsert property](https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/properties/devenv-delayedinsert-property)
- General Journal, Item Journal, and IC General Journal all set `DelayedInsert = True` and `AutoSplitKey = True`. — [Page "General Journal"](https://learn.microsoft.com/en-us/dynamics365/business-central/application/base-application/page/microsoft.finance.generalledger.journal.general-journal); [Page "Item Journal"](https://learn.microsoft.com/en-us/dynamics365/business-central/application/base-application/page/microsoft.inventory.journal.item-journal); [Page "IC General Journal"](https://learn.microsoft.com/en-us/dynamics365/business-central/application/base-application/page/microsoft.intercompany.journal.ic-general-journal)
- `AutoSplitKey`: “a key is automatically created for a new record placed between the current record and the previous record.” The new key “is set to a value halfway between the keys of the surrounding records.” The last primary-key field must be integer, big integer, GUID, or decimal. From Business Central 2025 release wave 1, inserting before the first row “a negative key value may [be] generated.” AutoSplitKey “doesn't currently generate 0.” — [AutoSplitKey property](https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/properties/devenv-autosplitkey-property)
- Journal lines stay editable until posting: “The information that you enter in a journal is temporary and can be changed while it's in the journal. When you post the journal, the information is transferred to entries on individual accounts, where it can't be changed.” — [Working with general journals](https://learn.microsoft.com/en-us/dynamics365/business-central/ui-work-general-journals)

Copy-above:

- F8 quotes in the shortcut section. Editable lists only. — [Keyboard Shortcuts](https://learn.microsoft.com/en-us/dynamics365/business-central/keyboard-shortcuts)

### Inferences

- On a General Journal, a new row is not written when the clerk leaves the primary-key field. `DelayedInsert = True` defers the insert until the clerk leaves the record. Microsoft does not say which key counts as leaving the record.
- The classic NAV row indicator (asterisk for new, pencil for the row being edited) is not what the current red asterisk or Edit list icon are. Those are different controls.

### Gaps

- No current user article describes a star, asterisk, or pencil drawn on the journal row itself. The row-glyph behavior is undocumented here.
- No user article says that moving off the last journal row creates a new line, or which key does it. Alt+N and Ctrl+Insert are the documented insert keys, and they are not tied by name to the General Journal page.
- No user article says changes save when you leave a cell. DelayedInsert only covers the first insert.

## How do date formulas (t, w, c, cm, cq, cy, +1D, closing dates) and in-cell calculators work on journals?

### Takeaway

`t` and `w` are date-entry codes (today and work date), not formula letters. `c` immediately before a date, as in `C123101`, marks a closing date. Date formulas use C, D, W, M, Q, and Y: `CM` is the last day of the current month, `10D` is ten days from today, and recurring journals store formulas such as `1M` and `1D+1M-1D` in Recurring Frequency. The in-cell calculator is typing `19+19` or `12*4` into a quantity field, including item journal Quantity. `cq`, `cy`, and a leading `+1D` are not given as their own examples on the user date page.

### Cited Findings

Date entry codes (any date field, including the shortcut page):

- Shortcut page, “Keyboard shortcuts in date fields”: `t` “Enter the current date. The t stands for today.” `w` “Enter the work date.” — [Keyboard Shortcuts](https://learn.microsoft.com/en-us/dynamics365/business-central/keyboard-shortcuts)
- Enter data code table: `t` today’s system date; `w` the work date from My Settings; `p` an accounting period (`p` first, `p2` second); `c` “the date after c is a closing date—for example, C123101.” — [Enter data](https://learn.microsoft.com/en-us/dynamics365/business-central/ui-enter-data)
- Closing date, fuller rule: “A closing date technically is between two dates, for example between December 31 and January 1.” “To specify that a date is a closing date, put C just before the date, such as C123101. Use this format in combination with all the date patterns.” — [Entering dates and times](https://learn.microsoft.com/en-us/dynamics365/business-central/ui-enter-date-ranges)
- Work date: enter `w` or a leading part of the word “work” in the user’s language. If no work date is set, today is used. Today can be `t` or `tod` unless that prefix is another word. Weekday-plus-week uses the work-date year: `Mon25` is Monday of week 25. `t` cannot mean Tuesday or Thursday because `t` already means today. Week numbers are ISO 8601. — [Entering dates and times](https://learn.microsoft.com/en-us/dynamics365/business-central/ui-enter-date-ranges)
- Digit entry: two digits are the day plus the work date’s month and year; four digits are day and month plus the work date’s year (region decides day/month order). Years from 01/01/1950 through 12/31/2049 can be two digits; otherwise four. On-premises range can differ via `CalendarTwoDigitYearMax`. Any separator is accepted and then normalized to the region. — [Entering dates and times](https://learn.microsoft.com/en-us/dynamics365/business-central/ui-enter-date-ranges); same digit rules on [Enter data](https://learn.microsoft.com/en-us/dynamics365/business-central/ui-enter-data)
- Calendar picker: activate the calendar icon or Ctrl+Home, except Ctrl+Home in a grid jumps to the first row (quoted in the focus section). — [Entering dates and times](https://learn.microsoft.com/en-us/dynamics365/business-central/ui-enter-date-ranges); [Keyboard Shortcuts](https://learn.microsoft.com/en-us/dynamics365/business-central/keyboard-shortcuts)
- Accounting period `p` / `period` plus a number is relative to the fiscal year of the work date. Example: work date 03/21/22, `p1` or `p` is the first period of fiscal 2022. Periods come from the Accounting Periods page. — [Entering dates and times](https://learn.microsoft.com/en-us/dynamics365/business-central/ui-enter-date-ranges)

Date formulas (calculation fields and filters, not the same as `t`/`w`):

- “A date formula is a short, abbreviated combination of letters and numbers.” Maximum 20 characters. Letters: C Current, D Day(s), W Week(s), M Month(s), Q Quarter(s), Y Year(s). A number cannot be larger than 9999. — [Entering dates and times](https://learn.microsoft.com/en-us/dynamics365/business-central/ui-enter-date-ranges)
- Documented expressions: `CW` current week; `CM` “Current month (last day of the month)”; `10D` “10 days from today”; `2W` “2 weeks from today”; `D10` “The next 10th day of a month”; `WD4` “The next 4th day of a week (Thursday)”; `CM+10D` “Current month + 10 days”; `-1Y` “1 year ago from today.” — [Entering dates and times](https://learn.microsoft.com/en-us/dynamics365/business-central/ui-enter-date-ranges)
- “In all data formula fields, one day is automatically included to cover today.” `1W` “is actually eight days because today is included.” A seven-day period including the start date is `6D` or `1W-1D`. — [Entering dates and times](https://learn.microsoft.com/en-us/dynamics365/business-central/ui-enter-date-ranges)
- If a location uses a base calendar, a formula in a field such as Shipping Time uses working days: “1W means seven working days.” — [Entering dates and times](https://learn.microsoft.com/en-us/dynamics365/business-central/ui-enter-date-ranges)
- Journal use: “The date formula in the recurring frequency field in recurring journals determines how often the entry on the journal line posts.” — [Entering dates and times](https://learn.microsoft.com/en-us/dynamics365/business-central/ui-enter-date-ranges)
- Recurring Frequency “must be filled in.” `1M` means every month, and after posting “the date in the Posting Date field will be updated to the same date in the next month.” Last day of every month: either post the first entry on a month-end with `1D+1M-1D`, or post on any day with `1M+CM`. — [Working with general journals](https://learn.microsoft.com/en-us/dynamics365/business-central/ui-work-general-journals)
- AL `CalcDate` (developer syntax, angle brackets, not the clerk’s field format) documents `CQ`: `'<CQ+1M-10D>'` is commented “Current quarter + 1 month - 10 days.” Reference date defaults to the system date. — [System.CalcDate](https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/system/system-calcdate-dateformula-date-method)
- `CQ` and `CY` do not appear as rows in the user-facing expression table. That table only spells out `CW` and `CM`, while Q and Y are in the letter table. — [Entering dates and times](https://learn.microsoft.com/en-us/dynamics365/business-central/ui-enter-date-ranges)

In-cell calculator (quantity fields, explicitly an item journal):

- “When entering numbers into quantity fields, such as the Quantity field on an item journal line, you can enter the formula instead of the sum quantity.” Examples: `19+19` becomes 38; `41-9` becomes 32; `12*4` becomes 48; `12/4` becomes 3. — [Enter data](https://learn.microsoft.com/en-us/dynamics365/business-central/ui-enter-data)
- Negatives: `-20.5` or `20.5-` both store -20.5. “If the last character of the expression is a + or a -, the entire expression will be recorded with that sign.” Example: “10-20+ will result in 10 and not -10.” — [Enter data](https://learn.microsoft.com/en-us/dynamics365/business-central/ui-enter-data)
- Numeric keypad: Alt+Decimal separator toggles the keypad’s decimal key between a period and the region’s decimal separator. — [Keyboard Shortcuts](https://learn.microsoft.com/en-us/dynamics365/business-central/keyboard-shortcuts); explained with the date-range example `01/01/2022..04/01/2022` on [Enter data](https://learn.microsoft.com/en-us/dynamics365/business-central/ui-enter-data)

### Inferences

- `+1D` is the same shape as the documented `10D` / `CM+10D` / `1D+1M-1D` pieces, but the user page never shows a leading plus on a single term. The safe documented form for “one day from today” is `1D`, by the `10D` rule.
- `CQ` and `CY` follow “C plus a unit,” and AL documents `CQ`. The clerk-facing table does not list them, so a datasheet should not present `CQ`/`CY` as user-doc examples without that caveat.
- `t` and `w` belong in date fields such as Posting Date. `CM` and `1M` belong in date-formula fields such as Recurring Frequency. Using `t` inside a formula is not documented.

### Gaps

- No page says the quantity formula (`19+19`) also runs in Amount on a general journal. The example is item journal Quantity and “quantity fields.”
- No page documents a separate calculator window or a shortcut that opens one.
- No user example of `+1D`, `CQ`, or `CY` as a typed field value.

## How do lookups, option dropdowns, and AssistEdit work from a cell?

### Takeaway

Alt+Down opens either a lookup (pick one related record) or a dropdown (pick one fixed option). Typing filters that list. Assist edit is a custom picker; Ctrl+Alt+Down opens the related card or list for a lookup or assist-edit value. NAV used F4 for lookup and Shift+F4 for the ellipsis AssistEdit; those two keys are not in the current Business Central shortcut article.

### Cited Findings

- “When you start to type characters in a field, you'll see a dropdown list that shows possible field values. The list changes as you type more characters, and you can select the correct value when it's displayed.” — [Enter data](https://learn.microsoft.com/en-us/dynamics365/business-central/ui-enter-data)
- The down-arrow button has two functions. “Lookup: Displays information from another table that you can enter in the field. You can select one piece of data at a time.” “Dropdown: Displays the set of options that exist for the field. You can select only one of the options.” — [Enter data](https://learn.microsoft.com/en-us/dynamics365/business-central/ui-enter-data)
- Picker table: Date picker is a regional calendar for one date. Dropdown is “fixed values or reference records from another table.” Switch or checkbox is Yes/No and “is always displayed as a checkbox in lists.” “Assist edit: Some fields provide custom pickers that are suited to looking up and choosing the best value for that field, such as a pop-up window.” — [Enter data](https://learn.microsoft.com/en-us/dynamics365/business-central/ui-enter-data)
- Alt+Down arrow opens “a dropdown list or look up a value for a field.” — [Keyboard Shortcuts](https://learn.microsoft.com/en-us/dynamics365/business-central/keyboard-shortcuts)
- Ctrl+Alt+Down arrow opens “a related record's card or list page filtered to the value in the lookup or assist edit field.” — [Keyboard Shortcuts](https://learn.microsoft.com/en-us/dynamics365/business-central/keyboard-shortcuts)
- Alt+Right arrow looks up “additional information or underlying values” when Alt+Down is already used for something else on that field. — [Keyboard Shortcuts](https://learn.microsoft.com/en-us/dynamics365/business-central/keyboard-shortcuts)
- Esc “Close the current page or dropdown list.” — [Keyboard Shortcuts](https://learn.microsoft.com/en-us/dynamics365/business-central/keyboard-shortcuts)
- Recent values: some lookups let you choose Recent to show recently used values first, or Search to return to the normal order. Typing while Recent is showing “switches back to search results.” Choosing Recent again keeps the typed text, but “The typed text doesn't filter the recent values.” “Column sorting isn't available while the lookup shows recent values.” — [Enter data](https://learn.microsoft.com/en-us/dynamics365/business-central/ui-enter-data)
- Inside an open lookup you filter “like any other list”: choose the top of the list or Shift+F3. — [Enter data](https://learn.microsoft.com/en-us/dynamics365/business-central/ui-enter-data) (the article’s “Filtering line items” paragraph)
- NAV-only: F4 “Drop-down or look up to select.” Shift+F4 “Open a lookup window (from an ellipsis button).” Shift+F8 “Drop-down or look up to view.” Alt+Enter “Move to the field below without opening the drop-down menu.” — [NAV 2018 Windows client shortcuts](https://learn.microsoft.com/en-us/previous-versions/dynamicsnav-2018/keyboard-shortcuts-windows-client)
- NAV-only triggers: F4 fires OnLookup; Shift+F4 fires OnAssistEdit; Shift+F8 fires OnDrillDown. — [Page and Action Triggers (NAV 2018)](https://learn.microsoft.com/en-us/previous-versions/dynamicsnav-2018-developer/Page-and-Action-Triggers)

### Inferences

- Option fields and lookup fields share Alt+Down. The button type, not a different shortcut, decides whether the list is fixed options or another table.
- Current AssistEdit keyboard path that Microsoft names is Ctrl+Alt+Down (open the related card/list for the value already in the field) plus the on-screen assist-edit button. Shift+F4 is NAV Windows client only.

### Gaps

- No current shortcut is documented for “open the AssistEdit pop-up” itself. Ctrl+Alt+Down is “filtered to the value in the lookup or assist edit field,” which is drill-through, not “open the picker.”
- No page says whether Enter commits a highlighted lookup row or moves to the next Quick Entry field while the lookup is open.
- No page says option dropdowns accept the first matching letter as a type-ahead key. The documented behavior is a list that changes as you type.

## What happens on validation errors, and how does the user move past a bad cell?

### Takeaway

The client checks a value when focus leaves the field, not while you type. Alt+Up shows the validation error; Esc returns to the input. Journal batches can also show a Check Journal FactBox and only recheck when the batch loads or you move to another line. Microsoft does not document a key that bypasses a bad cell, and it does not say focus is trapped there.

### Cited Findings

- “After you specify a value, Business Central will only check that it's valid after you click outside the field or set focus to another element, such as the next field.” — [Enter data](https://learn.microsoft.com/en-us/dynamics365/business-central/ui-enter-data)
- Alt+Up arrow displays “the field tooltip, validation error, Copilot's suggested value, or information about why an AI agent selected that value.” If more than one exists, the shortcut toggles. Esc “close this information and return to the field input box.” — [Keyboard Shortcuts](https://learn.microsoft.com/en-us/dynamics365/business-central/keyboard-shortcuts)
- Inline validation dialogs have a Share icon; error dialogs have Share details. “Select the Copy details icon on inline validation errors or in an error message” for field names and technical detail. An action on a platform error “take[s] you directly to the page where a field is causing the error.” No link is added “If the error is on the page that's currently open.” — [Warnings and error messages](https://learn.microsoft.com/en-us/dynamics365/business-central/product-error-messages)
- Real-time document/journal check is optional. My Notifications: journals use “Enabled” on “Enable Data Check.” The FactBox is “Check Document or Check Journal” and “lists the errors on the page so you can quickly resolve them.” — [Enter data](https://learn.microsoft.com/en-us/dynamics365/business-central/ui-enter-data); same paragraph on [Working with general journals](https://learn.microsoft.com/en-us/dynamics365/business-central/ui-work-general-journals)
- Batch background check is separate. On General Journal Batches, “Background Error Check” validates finance journals “while you're working on them.” “Validation happens when you load a finance journal batch, and when you choose another journal line.” The FactBox Issues tile opens the list. “Show Lines with Issues” and “Show All Lines” toggle the grid. — [Working with general journals](https://learn.microsoft.com/en-us/dynamics365/business-central/ui-work-general-journals)
- A mandatory field’s red asterisk does not block leaving the field. You can “continue to other fields or close the page.” The block comes when you run the process that needs the value, “for example… when you post.” — [Enter data](https://learn.microsoft.com/en-us/dynamics365/business-central/ui-enter-data)
- NAV-only: Shift+F1 “View error message.” — [NAV 2018 Windows client shortcuts](https://learn.microsoft.com/en-us/previous-versions/dynamicsnav-2018/keyboard-shortcuts-windows-client)

### Inferences

- Moving to the next cell (Tab, Enter, or click) is both the commit and the moment of validation. If the value is rejected, the documented recovery is read the error with Alt+Up, change the value, and leave the field again. That sequence is assembled from two pages; neither page says the focus stays put after a failed validate.
- Background journal check is line-oriented: it runs when you select another line, so a clerk can be on a different line while the FactBox still lists issues. That is not the same as an inline field error.

### Gaps

- No Microsoft page states that an invalid cell traps the cursor, or names a key (Esc, Ctrl+Z, or otherwise) that discards the bad edit and moves on.
- No page says whether a journal inline error is a dialog you must dismiss before the next cell, or only a message Alt+Up can show.
- NAV Shift+F1 “View error message” has no matching entry in the current Business Central shortcut article. BC’s documented “show the error” key is Alt+Up.
