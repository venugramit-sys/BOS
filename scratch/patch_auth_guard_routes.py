with open('crates/bosc-runtime/src/auth_guard.rs', 'rb') as f:
    data = f.read()
old = b'    if (path.starts_with("/api/v1/pages/LoginPage/triggers/")\r\n        || path.starts_with("/api/v1/pages/SignIn/triggers/"))'
new = b'    if (path.starts_with("/api/v1/pages/LoginPage/triggers/")\r\n        || path.starts_with("/api/pages/LoginPage/triggers/")\r\n        || path.starts_with("/api/v1/pages/SignIn/triggers/")\r\n        || path.starts_with("/api/pages/SignIn/triggers/"))'
assert old in data, 'old not found'
data = data.replace(old, new)
with open('crates/bosc-runtime/src/auth_guard.rs', 'wb') as f:
    f.write(data)
print('Successfully patched auth_guard.rs')
