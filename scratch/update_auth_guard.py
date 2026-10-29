with open('crates/bosc-runtime/src/auth_guard.rs', 'r', encoding='utf-8') as f:
    c = f.read()

target = """    let exact = [
        "/api/health",
        "/api/auth/status",
        "/api/auth/bootstrap",
        "/api/auth/login",
        "/api/detect-db",
        "/api/node/info",
    ];
    if exact.contains(&path) {
        return true;
    }
    if path == "/api/auth/config" && method == Method::GET {
        return true;
    }
    if path.starts_with("/api/setup/") {
        return true;
    }"""

repl = """    let exact = [
        "/api/health",
        "/api/auth/status",
        "/api/auth/bootstrap",
        "/api/auth/login",
        "/api/detect-db",
        "/api/node/info",
        "/api/v1/pages/manifest",
        "/api/v1/pages/manifest/stream",
    ];
    if exact.contains(&path) {
        return true;
    }
    if path == "/api/auth/config" && method == Method::GET {
        return true;
    }
    if path.starts_with("/api/setup/") {
        return true;
    }
    // Unauthenticated login pages (LoginPage & SignIn) can run their sign-in action triggers
    if (path.starts_with("/api/v1/pages/LoginPage/triggers/")
        || path.starts_with("/api/v1/pages/SignIn/triggers/"))
        && method == Method::POST
    {
        return true;
    }"""

test_target = """        for p in ["/api/health", "/api/auth/status", "/api/auth/login", "/api/auth/bootstrap",
                  "/api/setup/status", "/api/setup/provision", "/api/detect-db", "/api/databases",
                  "/api/node/info", "/", "/index.html"] {
            assert!(is_public_route(&Method::GET, p), "{p} should be public");
        }"""

test_repl = """        for p in ["/api/health", "/api/auth/status", "/api/auth/login", "/api/auth/bootstrap",
                  "/api/setup/status", "/api/setup/provision", "/api/detect-db", "/api/databases",
                  "/api/node/info", "/api/v1/pages/manifest", "/api/v1/pages/manifest/stream",
                  "/", "/index.html"] {
            assert!(is_public_route(&Method::GET, p), "{p} should be public");
        }"""

is_crlf = '\r\n' in c
if is_crlf:
    target = target.replace('\n', '\r\n')
    repl = repl.replace('\n', '\r\n')
    test_target = test_target.replace('\n', '\r\n')
    test_repl = test_repl.replace('\n', '\r\n')

assert target in c, "target not found"
c = c.replace(target, repl, 1)

assert test_target in c, "test_target not found"
c = c.replace(test_target, test_repl, 1)

with open('crates/bosc-runtime/src/auth_guard.rs', 'w', encoding='utf-8', newline='') as f:
    f.write(c)

print("Updated auth_guard.rs successfully")
