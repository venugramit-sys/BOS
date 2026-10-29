with open('ui/src/lib/services/errorService.js', 'r', encoding='utf-8') as f:
    content = f.read()

target = '''    // Set page-level banner if page controller is passed
    if (options.page) {
      options.page.error = norm.message;
      if (options.field) {
        options.page.fieldErrors = {
          ...options.page.fieldErrors,
          [options.field]: norm.message
        };
      }
    }'''

replacement = '''    // Set page-level banner if page controller is passed
    if (options.page) {
      options.page.error = norm.message;
      options.page.errorTitle = title;
      // In-field error attribution: If not explicitly supplied, infer field from message
      let errorField = options.field;
      if (!errorField) {
        const lower = norm.message.toLowerCase();
        if (lower.includes("user id") || lower.includes("username")) {
          errorField = "username";
        } else if (lower.includes("password")) {
          errorField = "password";
        }
      }
      if (errorField) {
        options.page.fieldErrors = {
          ...options.page.fieldErrors,
          [errorField]: norm.message
        };
      }
    }'''

assert target in content, "target not found in errorService.js"
content = content.replace(target, replacement)

with open('ui/src/lib/services/errorService.js', 'w', encoding='utf-8') as f:
    f.write(content)
print("errorService.js updated with field error inference and errorTitle!")
