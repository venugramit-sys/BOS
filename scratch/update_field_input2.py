with open('ui/src/lib/page-kit/FieldInput.svelte', 'r', encoding='utf-8') as f:
    content = f.read()

# 1. Error resolution across key, source, and name
target_err = '  const error = $derived(row ? "" : (p.fieldErrors[f.key] ?? ""));'
replacement_err = '  const error = $derived(row ? "" : (p.fieldErrors[f.key] ?? p.fieldErrors[f.source] ?? p.fieldErrors[c.name] ?? ""));'

# 2. Password type handling
target_type = 'type={f.control === "email" ? "email" : f.control === "phone" ? "tel" : f.control === "url" ? "url" : f.control === "masked" ? "password" : "text"}'
replacement_type = 'type={f.control === "email" ? "email" : f.control === "phone" ? "tel" : f.control === "url" ? "url" : (f.control === "masked" || f.control === "password" || f.key === "password_hash" || f.key === "password" || f.source === "passwordHash") ? "password" : "text"}'

for t, r in [(target_err, replacement_err), (target_type, replacement_type)]:
    t_crlf = t.replace('\n', '\r\n')
    r_crlf = r.replace('\n', '\r\n')
    if t in content:
        content = content.replace(t, r, 1)
        print("Replaced (LF)")
    elif t_crlf in content:
        content = content.replace(t_crlf, r_crlf, 1)
        print("Replaced (CRLF)")
    else:
        print(f"NOT found: {t[:30]}")

with open('ui/src/lib/page-kit/FieldInput.svelte', 'w', encoding='utf-8') as f:
    f.write(content)
print("FieldInput.svelte update complete")
