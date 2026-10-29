with open('ui/src/lib/page-kit/focus.js', 'r', encoding='utf-8') as f:
    content = f.read()

target = '''    // Normal Enter: advance to next editable field
    if (currentIndex !== -1 && currentIndex < editables.length - 1) {
      focusElement(editables[currentIndex + 1]);
      return;
    }

    // On last editable field: trigger or focus primary action button'''

replacement = '''    // If pressing Enter in a password field, immediately submit primary action (best-in-class login UX)
    const isPasswordField = active.tagName === 'INPUT' && (active.type === 'password' || active.name === 'password');
    if (isPasswordField) {
      const footer = container.querySelector?.('.bos-page-footer') || container;
      const primaryBtn =
        footer.querySelector('button.bos-btn-primary') ||
        footer.querySelector('button[variant="primary"]') ||
        footer.querySelector('button[type="submit"]') ||
        footer.querySelector('button:not([disabled]):not([tabindex="-1"])');

      if (primaryBtn && isControlVisible(primaryBtn)) {
        if (onPrimary) {
          onPrimary();
        } else {
          primaryBtn.click();
        }
        return;
      }
    }

    // Normal Enter: advance to next editable field
    if (currentIndex !== -1 && currentIndex < editables.length - 1) {
      focusElement(editables[currentIndex + 1]);
      return;
    }

    // On last editable field: trigger or focus primary action button'''

assert target in content, "target not found in focus.js"
content = content.replace(target, replacement)

with open('ui/src/lib/page-kit/focus.js', 'w', encoding='utf-8') as f:
    f.write(content)
print("focus.js updated successfully!")
