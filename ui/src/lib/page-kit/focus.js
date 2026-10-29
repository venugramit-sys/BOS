/**
 * Enterprise Form Focus & QuickEntry Manager (inspired by MS Dynamics NAV / Business Central & SAP ERP).
 * 
 * Capabilities:
 * - Automatically auto-focuses the first editable control when a page, card, or dialog mounts.
 * - Navigates to the next editable field on `Enter` (QuickEntry standard).
 * - Navigates to the previous editable field on `Shift+Enter`.
 * - Toggles switches and checkboxes on `Space`, but advances on `Enter`.
 * - Advances to the primary action button on the last field, or triggers primary action.
 * - Auto-centers input into view on mobile screens when virtual keyboard opens.
 * - Auto-focuses the invalid field when validation errors occur.
 * - Skips disabled, readonly, hidden, or non-tabbable elements.
 * - Auto-selects text on input focus for fast replacement typing (NAV QuickEntry standard).
 */

const EDITABLE_SELECTOR = [
  'input:not([type="hidden"]):not([disabled]):not([readonly]):not([tabindex="-1"])',
  'select:not([disabled]):not([tabindex="-1"])',
  'textarea:not([disabled]):not([readonly]):not([tabindex="-1"])',
  'button[role="switch"]:not([disabled]):not([aria-disabled="true"]):not([tabindex="-1"])',
  'button[role="checkbox"]:not([disabled]):not([aria-disabled="true"]):not([tabindex="-1"])',
  '[role="combobox"]:not([disabled]):not([aria-disabled="true"]):not([tabindex="-1"])'
].join(', ');

export function isControlVisible(el) {
  if (!el || typeof el.getBoundingClientRect !== 'function') return false;
  if (el.offsetParent === null && el.tagName !== 'BODY') return false;
  const style = window.getComputedStyle(el);
  if (style.display === 'none' || style.visibility === 'hidden' || style.opacity === '0') return false;
  const rect = el.getBoundingClientRect();
  return rect.width > 0 && rect.height > 0;
}

export function getVisibleEditables(container, { quickEntryOnly = false } = {}) {
  if (!container) return [];
  const root = container.querySelector?.('.bos-page-body') || container;
  const elements = Array.from(root.querySelectorAll(EDITABLE_SELECTOR));
  return elements.filter((el) => {
    if (!isControlVisible(el)) return false;
    if (quickEntryOnly && el.getAttribute('data-quick-entry') === 'false') return false;
    return true;
  });
}

export function focusElement(el, { selectText = true } = {}) {
  if (!el) return false;
  el.focus();
  if (selectText && typeof el.select === 'function' && el.type !== 'password') {
    try {
      el.select();
    } catch {
      // Ignored for input types that don't support selection range
    }
  }
  if (typeof el.scrollIntoView === 'function') {
    const isMobile = typeof window !== 'undefined' && window.innerWidth < 640;
    el.scrollIntoView({ behavior: 'smooth', block: isMobile ? 'center' : 'nearest' });
  }
  return true;
}

export function focusFirstEditable(container, opts = {}) {
  const list = getVisibleEditables(container);
  if (list.length > 0) {
    return focusElement(list[0], opts);
  }
  return false;
}

export function focusFieldError(container, fieldName) {
  if (!container || !fieldName) return false;
  const target = container.querySelector(
    `[name="${fieldName}"], [data-field="${fieldName}"], [id*="-${fieldName}-"], [id*="${fieldName}"]`
  );
  if (target && isControlVisible(target)) {
    return focusElement(target);
  }
  return false;
}

export function handleEnterpriseFormKeyDown(e, container, { onPrimary = null } = {}) {
  // Ignore in IME composition
  if (e.isComposing) return;
  // Ignore if modified by Alt or Meta/Command
  if (e.altKey || e.metaKey) return;

  if (e.key === 'Enter') {
    const active = document.activeElement;
    if (!active || !container || !container.contains(active)) return;

    // Multi-line textareas accept Enter for newlines unless Ctrl is pressed
    if (active.tagName === 'TEXTAREA' && !e.ctrlKey) {
      return;
    }

    // Standard buttons handle Enter natively
    if (active.tagName === 'BUTTON' && active.getAttribute('role') !== 'switch' && active.getAttribute('role') !== 'checkbox') {
      return;
    }

    // If active element is an editable data-entry field (or switch/checkbox), intercept Enter for NAV QuickEntry
    const editables = getVisibleEditables(container);
    const currentIndex = editables.indexOf(active);

    e.preventDefault();

    if (e.shiftKey) {
      // Shift+Enter: move to previous field
      if (currentIndex > 0) {
        focusElement(editables[currentIndex - 1]);
      } else if (editables.length > 0) {
        focusElement(editables[editables.length - 1]);
      }
      return;
    }

    // If pressing Enter in a password field, immediately submit primary action (best-in-class login UX)
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

    // On last editable field: trigger or focus primary action button
    const footer = container.querySelector?.('.bos-page-footer') || container;
    const primaryBtn =
      footer.querySelector('button.bos-btn-primary') ||
      footer.querySelector('button[variant="primary"]') ||
      footer.querySelector('button[type="submit"]') ||
      footer.querySelector('button:not([disabled]):not([tabindex="-1"])');

    if (primaryBtn && isControlVisible(primaryBtn)) {
      primaryBtn.focus();
      if (onPrimary) {
        onPrimary();
      } else {
        primaryBtn.click();
      }
    }
  }
}
