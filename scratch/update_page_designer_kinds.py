path = 'crates/bosc-runtime/src/page_designer.rs'
with open(path, 'r', encoding='utf-8') as f:
    text = f.read()

target = """    let kinds = [
        K::Area, K::Group, K::Grid, K::Fixed, K::Repeater, K::Part, K::SystemPart, K::Field, K::Label, K::Image, K::Spacer,
        K::Divider, K::CueGroup, K::Cue, K::Headline, K::UserControl, K::Tabs, K::Tab, K::Flex, K::Split, K::Card,
        K::WizardStep, K::Container,
    ];"""

replacement = """    let kinds = [
        K::Area, K::Group, K::Grid, K::Fixed, K::Repeater, K::Part, K::SystemPart, K::Field, K::Label, K::Image, K::Spacer,
        K::Divider, K::CueGroup, K::Cue, K::Headline, K::UserControl, K::Tabs, K::Tab, K::Flex, K::Split, K::Card,
        K::WizardStep, K::Container, K::Section, K::Header, K::Footer, K::Nav, K::Aside, K::Article, K::Action,
    ];"""

assert target in text, "target not found"
text = text.replace(target, replacement, 1)

with open(path, 'w', encoding='utf-8', newline='') as f:
    f.write(text)

print("Updated page_designer.rs successfully")
