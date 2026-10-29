with open('crates/bosc-runtime/src/auth_guard.rs', 'r', encoding='utf-8') as f:
    content = f.read()

target = '''        "/api/pages/SignIn",
        "/api/v1/pages/SignIn",
    ];'''

replacement = '''        "/api/pages/SignIn",
        "/api/v1/pages/SignIn",
        "/api/notifications/stream",
    ];'''

assert target in content, "target not found in auth_guard.rs"
content = content.replace(target, replacement)

with open('crates/bosc-runtime/src/auth_guard.rs', 'w', encoding='utf-8') as f:
    f.write(content)
print("auth_guard.rs updated with /api/notifications/stream successfully!")
