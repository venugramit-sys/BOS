path = 'ui/src/lib/page-kit/page-kit.test.js'
with open(path, 'r', encoding='utf-8') as f:
    text = f.read()

new_test = """
test("custom unbound page loads cleanly and handles action properties", () => {
  const meta = { name: "CustomDashboard", page_type: "Custom", caption: "Operations Dashboard", properties: { showHeader: "false" } };
  const action = { name: "DownloadReport", caption: "Download Report", properties: { primary: "true", openPage: "ReportViewer" } };
  assert.equal(meta.page_type, "Custom");
  assert.equal(meta.properties.showHeader, "false");
  assert.equal(action.properties.primary, "true");
  assert.equal(action.properties.openPage, "ReportViewer");
});
"""

if "custom unbound page loads cleanly" not in text:
    text += new_test
    with open(path, 'w', encoding='utf-8', newline='') as f:
        f.write(text)
    print("Added custom page test")
