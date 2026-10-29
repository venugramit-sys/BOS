with open('crates/bosc-runtime/src/auth_guard.rs', 'r', encoding='utf-8') as f:
    content = f.read()

target = '''    let exact = [
        "/api/health",
        "/api/auth/status",
        "/api/auth/bootstrap",
        "/api/auth/login",
        "/api/detect-db",
        "/api/node/info",
        "/api/v1/pages/manifest",
        "/api/v1/pages/manifest/stream",
    ];'''

replacement = '''    let exact = [
        "/api/health",
        "/api/auth/status",
        "/api/auth/bootstrap",
        "/api/auth/login",
        "/api/detect-db",
        "/api/node/info",
        "/api/v1/pages/manifest",
        "/api/v1/pages/manifest/stream",
        "/api/pages/LoginPage",
        "/api/v1/pages/LoginPage",
        "/api/pages/SignIn",
        "/api/v1/pages/SignIn",
    ];'''

assert target in content, "target not found in auth_guard.rs"
content = content.replace(target, replacement)

with open('crates/bosc-runtime/src/auth_guard.rs', 'w', encoding='utf-8') as f:
    f.write(content)
print("auth_guard.rs updated successfully!")
