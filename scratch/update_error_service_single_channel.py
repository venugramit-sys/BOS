with open('ui/src/lib/services/errorService.js', 'r', encoding='utf-8') as f:
    text = f.read()

# Normalize CRLF to LF for processing
has_crlf = '\r\n' in text
text_lf = text.replace('\r\n', '\n')

start_marker = "  error(err, options = {}) {"
end_marker = "  /**\n   * Shows a success toast"

start_idx = text_lf.find(start_marker)
assert start_idx != -1, "start_marker not found"

end_idx = text_lf.find(end_marker, start_idx)
assert end_idx != -1, "end_marker not found"

new_error_method = """  error(err, options = {}) {
    const norm = normalizeError(err);
    const title = options.title || "Action Failed";

    // SINGLE-CHANNEL POLICY:
    // When an active page context is present:
    // 1. Wherever possible -> In-Field error ONLY (suppress banner and toast).
    // 2. Otherwise -> Top Banner error ONLY (suppress toast).
    // 3. No page context -> Toast ONLY.
    if (options.page) {
      let errorField = options.field;

      if (!errorField) {
        const lower = norm.message.toLowerCase();

        // Check authentication / credential fields
        if (lower.includes("user id") || lower.includes("username")) {
          errorField = "username";
        } else if (lower.includes("password")) {
          errorField = "password";
        } else if (lower.includes("email")) {
          errorField = "email";
        }

        // Intelligently match against page controls if available
        if (!errorField && options.page.C && Array.isArray(options.page.C)) {
          for (const c of options.page.C) {
            const k = (c.field?.key || c.name || "").toLowerCase();
            const cap = (c.caption || c.field?.caption || "").toLowerCase();
            if (k && lower.includes(k)) {
              errorField = c.field?.key || c.name;
              break;
            }
            if (cap && lower.includes(cap)) {
              errorField = c.field?.key || c.name;
              break;
            }
          }
        }

        // Match against page variables or record keys
        if (!errorField && options.page.vars) {
          for (const vKey of Object.keys(options.page.vars)) {
            const k = vKey.toLowerCase();
            if (k && lower.includes(k)) {
              errorField = vKey;
              break;
            }
          }
        }
      }

      if (errorField) {
        // CHANNEL 1: In-field display ONLY.
        // Clear top banner and suppress toast so user sees a single, unified indicator.
        options.page.fieldErrors = {
          ...options.page.fieldErrors,
          [errorField]: norm.message
        };
        options.page.error = "";
        options.page.errorTitle = "";
        return { channel: "infield", field: errorField, message: norm.message };
      }

      // CHANNEL 2: Top banner display ONLY (for non-field errors on a page).
      // Clear field errors and suppress toast.
      options.page.error = norm.message;
      options.page.errorTitle = title;
      return { channel: "banner", message: norm.message };
    }

    // CHANNEL 3: Toast display ONLY (for background or system operations with no active page)
    return notificationStore.dispatch({
      severity: "error",
      target: options.target || "toast",
      title,
      message: norm.message,
      code: norm.code,
      details: norm.details,
      field: options.field,
      duration_ms: options.duration_ms ?? 8000
    });
  },

"""

result = text_lf[:start_idx] + new_error_method + text_lf[end_idx:]

if has_crlf:
    result = result.replace('\n', '\r\n')

with open('ui/src/lib/services/errorService.js', 'w', encoding='utf-8') as f:
    f.write(result)

print("errorService.js updated with single-channel policy")
