with open('ui/src/lib/services/errorService.js', 'rb') as f:
    content = f.read().decode('utf-8')

target = """      // In-field error attribution: If not explicitly supplied, infer field from message
      let errorField = options.field;
      if (!errorField) {
        const lower = norm.message.toLowerCase();
        if (lower.includes("user id") || lower.includes("username")) {
          errorField = "username";
        } else if (lower.includes("password")) {
          errorField = "password";
        }
      }"""

replacement = """      // In-field error attribution: If not explicitly supplied, infer field from message
      let errorField = options.field;
      if (!errorField) {
        const lower = norm.message.toLowerCase();
        // Combined credential failures (e.g. "Invalid User ID or Password.") are form-level
        // errors and must NOT be attributed to a single field (OWASP / enterprise UX).
        const isCombinedCredError =
          (lower.includes("user id") || lower.includes("username")) &&
          lower.includes("password");

        if (!isCombinedCredError) {
          if (lower.includes("user id") || lower.includes("username")) {
            errorField = "username";
          } else if (lower.includes("password")) {
            errorField = "password";
          }
        }
      }"""

# Normalize CRLF for matching if needed
if target not in content and target.replace('\n', '\r\n') in content:
    target = target.replace('\n', '\r\n')
    replacement = replacement.replace('\n', '\r\n')

assert target in content, "target string not found"
content = content.replace(target, replacement, 1)

with open('ui/src/lib/services/errorService.js', 'wb') as f:
    f.write(content.encode('utf-8'))

print("errorService.js successfully updated.")
