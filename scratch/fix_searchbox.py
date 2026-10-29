with open('scratch/verify_all_ux_dx_fixes.mjs', 'r', encoding='utf-8') as f:
    content = f.read()

content = content.replace("page.locator('input[role=\"searchbox\"]').isVisible()", "page.locator('input[placeholder*=\"Type to filter\"]').isVisible()")
content = content.replace("page.locator('input[role=\"searchbox\"]').first()", "page.locator('input[placeholder*=\"Type to filter\"]').first()")

with open('scratch/verify_all_ux_dx_fixes.mjs', 'w', encoding='utf-8') as f:
    f.write(content)
print("verify_all_ux_dx_fixes.mjs updated")
