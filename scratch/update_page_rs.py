import re

path = 'crates/bosc-ir/src/page.rs'
with open(path, 'r', encoding='utf-8') as f:
    text = f.read()

# 1. Update PageType enum
old_pt = """    Api,
    Board,
    Timeline,
}"""
new_pt = """    Api,
    Board,
    Timeline,
    Custom,
}"""
assert old_pt in text, "old_pt not found"
text = text.replace(old_pt, new_pt, 1)

# 2. Update PageType::ALL
old_all = """        PageType::Board,
        PageType::Timeline,
    ];"""
new_all = """        PageType::Board,
        PageType::Timeline,
        PageType::Custom,
    ];"""
assert old_all in text, "old_all not found"
text = text.replace(old_all, new_all, 1)
text = text.replace("pub const ALL: [PageType; 13] =", "pub const ALL: [PageType; 14] =", 1)

# 3. Update PageType::parse
old_parse = """            "board" | "kanban" => PageType::Board,
            "timeline" => PageType::Timeline,
            _ => return None,"""
new_parse = """            "board" | "kanban" => PageType::Board,
            "timeline" => PageType::Timeline,
            "custom" | "portal" | "canvas" | "blank" | "page" => PageType::Custom,
            _ => return None,"""
assert old_parse in text, "old_parse not found"
text = text.replace(old_parse, new_parse, 1)

# 4. Update PageType::as_str
old_as_str = """            PageType::Board => "Board",
            PageType::Timeline => "Timeline",
        }"""
new_as_str = """            PageType::Board => "Board",
            PageType::Timeline => "Timeline",
            PageType::Custom => "Custom",
        }"""
assert old_as_str in text, "old_as_str not found"
text = text.replace(old_as_str, new_as_str, 1)

# 5. Update PageType::allows
old_allows = """        match kind {
            K::Other(_) => false,
            k if k.is_directive() => true,
            K::Repeater => self.is_list_like() || matches!(self, PageType::Document | PageType::NavigatePage | PageType::Api),
            K::CueGroup | K::Cue => matches!(self, PageType::RoleCenter | PageType::CardPart),
            K::WizardStep => matches!(self, PageType::NavigatePage),
            K::Part | K::SystemPart => !self.is_part() || matches!(self, PageType::CardPart),
            K::Headline => matches!(self, PageType::RoleCenter),
            _ => true,
        }"""
new_allows = """        match kind {
            K::Other(_) => false,
            k if k.is_directive() => true,
            K::Action => true,
            K::Repeater => self.is_list_like() || matches!(self, PageType::Document | PageType::NavigatePage | PageType::Api | PageType::Custom),
            K::CueGroup | K::Cue => matches!(self, PageType::RoleCenter | PageType::CardPart | PageType::Custom),
            K::WizardStep => matches!(self, PageType::NavigatePage | PageType::Custom),
            K::Part | K::SystemPart => !self.is_part() || matches!(self, PageType::CardPart | PageType::Custom),
            K::Headline => matches!(self, PageType::RoleCenter | PageType::Custom),
            _ => true,
        }"""
assert old_allows in text, "old_allows not found"
text = text.replace(old_allows, new_allows, 1)

# 6. Update PageControlKind enum
old_pck = """    Aside,
    Article,
    AddFirst,"""
new_pck = """    Aside,
    Article,
    Action,
    AddFirst,"""
assert old_pck in text, "old_pck not found"
text = text.replace(old_pck, new_pck, 1)

# 7. Update PageControlKind::parse
old_pck_parse = """            "tabs" => K::Tabs,
            "tab" => K::Tab,"""
new_pck_parse = """            "action" | "button" | "cta" => K::Action,
            "tabs" => K::Tabs,
            "tab" => K::Tab,"""
assert old_pck_parse in text, "old_pck_parse not found"
text = text.replace(old_pck_parse, new_pck_parse, 1)

# 8. Update PageControlKind::as_str
old_pck_as_str = """            K::Aside => "aside",
            K::Article => "article",
            K::AddFirst => "addFirst","""
new_pck_as_str = """            K::Aside => "aside",
            K::Article => "article",
            K::Action => "action",
            K::AddFirst => "addFirst","""
assert old_pck_as_str in text, "old_pck_as_str not found"
text = text.replace(old_pck_as_str, new_pck_as_str, 1)

with open(path, 'w', encoding='utf-8', newline='') as f:
    f.write(text)

print("Updated page.rs successfully")
