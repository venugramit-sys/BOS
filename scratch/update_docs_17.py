path = 'docs/17_page_compiler_and_designer.md'
with open(path, 'r', encoding='utf-8') as f:
    text = f.read()

target1 = """    pageType = Card;                       // Card, List, Document, ListPart, CardPart, Worksheet,
                                           // StandardDialog, ConfirmationDialog, NavigatePage,
                                           // RoleCenter, Api, Board, Timeline"""

replacement1 = """    pageType = Card;                       // Card, List, Document, ListPart, CardPart, Worksheet,
                                           // StandardDialog, ConfirmationDialog, NavigatePage,
                                           // RoleCenter, Api, Board, Timeline, Custom"""

assert target1 in text, "target1 not found"
text = text.replace(target1, replacement1, 1)

target2 = "  | `PageType` | Card, List, Document, ListPart, CardPart, Worksheet, StandardDialog, ConfirmationDialog, NavigatePage, RoleCenter, Api, Board, Timeline |"
replacement2 = "  | `PageType` | Card, List, Document, ListPart, CardPart, Worksheet, StandardDialog, ConfirmationDialog, NavigatePage, RoleCenter, Api, Board, Timeline, Custom |"
assert target2 in text, "target2 not found"
text = text.replace(target2, replacement2, 1)

target3 = "  | `PageControlKind` | area, group, grid, fixed, repeater, part, systempart, field, label, image, spacer, divider, cuegroup, cue, headline, usercontrol, tabs, tab, flex, split, card, wizardstep, container, plus directives |"
replacement3 = "  | `PageControlKind` | area, group, grid, fixed, repeater, part, systempart, field, label, image, spacer, divider, cuegroup, cue, headline, usercontrol, tabs, tab, flex, split, card, wizardstep, container, section, header, footer, nav, aside, article, action, plus directives |"
assert target3 in text, "target3 not found"
text = text.replace(target3, replacement3, 1)

with open(path, 'w', encoding='utf-8', newline='') as f:
    f.write(text)

print("Updated docs/17_page_compiler_and_designer.md successfully")
