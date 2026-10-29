path_fixture = 'crates/bosc-codegen-svelte/tests/fixtures/pages.bos'
with open(path_fixture, 'r', encoding='utf-8') as f:
    text = f.read()

custom_page = """
page CustomDashboard {
    pageType = Custom;
    caption = "Operations Dashboard";
    layout {
        area(Content) {
            section(Hero) {
                headline(Title) { caption = "Executive Dashboard"; }
                label(Desc) { caption = "Real-time metrics and operations"; }
                action(Refresh) { caption = "Refresh"; primary = true; }
            }
            grid(Metrics) {
                cols = 2;
                card(CardA) { caption = "Revenue"; label(ValA) { caption = "$100,000"; } }
                card(CardB) { caption = "Users"; label(ValB) { caption = "2,400"; } }
            }
            flex(ActionsRow) {
                action(DownloadReport) { caption = "Download Report"; }
            }
        }
    }
}
"""

if "page CustomDashboard" not in text:
    text += custom_page
    with open(path_fixture, 'w', encoding='utf-8', newline='') as f:
        f.write(text)
    print("Added CustomDashboard to pages.bos")

path_golden = 'crates/bosc-codegen-svelte/tests/golden.rs'
with open(path_golden, 'r', encoding='utf-8') as f:
    text_g = f.read()

target = '    "CustomerWorkspace",\n];'
replacement = '    "CustomerWorkspace",\n    "CustomDashboard",\n];'
if target in text_g:
    text_g = text_g.replace(target, replacement, 1)
    with open(path_golden, 'w', encoding='utf-8', newline='') as f:
        f.write(text_g)
    print("Updated golden.rs")
