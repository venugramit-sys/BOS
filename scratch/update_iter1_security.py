with open('crates/bosc-runtime/tests/iter1_security.rs', 'r', encoding='utf-8') as f:
    c = f.read()

target = '''    for p in ["/api/entities/user", "/api/entities/role", "/api/query", "/api/tenants",
              "/api/codeunits/AuthManager/run", "/api/meta/objects", "/api/v1/pages/manifest"] {
        assert!(!is_public_route(&Method::GET, p), "{p} must require auth");
    }
    for p in ["/api/health", "/api/auth/status", "/api/auth/login", "/api/setup/status"] {
        assert!(is_public_route(&Method::GET, p), "{p} must stay public");
    }'''

repl = '''    for p in ["/api/entities/user", "/api/entities/role", "/api/query", "/api/tenants",
              "/api/codeunits/AuthManager/run", "/api/meta/objects", "/api/script/apply"] {
        assert!(!is_public_route(&Method::GET, p), "{p} must require auth");
    }
    for p in ["/api/health", "/api/auth/status", "/api/auth/login", "/api/setup/status",
              "/api/v1/pages/manifest", "/api/v1/pages/manifest/stream"] {
        assert!(is_public_route(&Method::GET, p), "{p} must stay public");
    }'''

is_crlf = '\r\n' in c
if is_crlf:
    target = target.replace('\n', '\r\n')
    repl = repl.replace('\n', '\r\n')

assert target in c, "target not found"
c = c.replace(target, repl, 1)

with open('crates/bosc-runtime/tests/iter1_security.rs', 'w', encoding='utf-8', newline='') as f:
    f.write(c)

print("Updated iter1_security.rs successfully")
