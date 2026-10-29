// @ts-check
import { notificationStore } from "./notificationStore.svelte.js";

/**
 * Enterprise Centralized Error & Notification Service
 * (Conforms to Microsoft Dynamics NAV / Business Central & SAP Notification Architecture)
 * 
 * Supports:
 * - Transient Toasts (non-blocking status updates)
 * - Page-level contextual error banners
 * - In-field validation error markers
 * - Modal Alert Popups (with copyable diagnostics & stack traces)
 * - Unified HTTP, QuickJS engine, and business rule error normalization
 */

/**
 * Normalizes any error object, string, or HTTP response into a structured notification payload.
 * @param {any} err
 * @returns {{ message: string, code?: string, details?: string }}
 */
export function normalizeError(err) {
  if (!err) {
    return { message: "An unexpected error occurred." };
  }

  let raw = "";
  let code = undefined;
  let details = undefined;

  if (typeof err === "string") {
    raw = err;
  } else if (typeof err === "object" && err !== null) {
    raw = err.error || err.message || err.reason || err.statusText || "";
    code = err.code || (err.status ? String(err.status) : undefined);
    details = err.details || err.stack || undefined;
    if (!raw && typeof err.toString === "function" && err.toString() !== "[object Object]") {
      raw = err.toString();
    }
  } else {
    raw = String(err);
  }

  raw = raw || "Operation failed.";

  // Extract clean human-friendly message from QuickJS / engine exception wrappers:
  // e.g. "Business logic exception in 'AuthConfiguration.action.SignIn' (Error): Invalid User ID or Password. [stack: ...]"
  let cleanMessage = raw;
  if (raw.includes("(Error): ")) {
    const parts = raw.split("(Error): ");
    const after = parts[1] || "";
    cleanMessage = after.split("[stack:")[0].trim();
    if (!details) details = raw;
  } else if (raw.startsWith("Error: ")) {
    cleanMessage = raw.replace(/^Error:\s*/, "");
    if (!details) details = raw;
  }

  return {
    message: cleanMessage,
    code,
    details
  };
}

export const notify = {
  /**
   * Reports an error through toasts, page banners, or field highlights.
   * @param {any} err - Error object, string, or backend response
   * @param {{
   *   title?: string,
   *   target?: 'toast' | 'popup' | 'infield' | 'banner',
   *   field?: string,
   *   page?: any,
   *   duration_ms?: number
   * }} [options]
   */
  error(err, options = {}) {
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

  /**
   * Shows a success toast
   * @param {string} message
   * @param {{ title?: string, duration_ms?: number }} [options]
   */
  success(message, options = {}) {
    return notificationStore.dispatch({
      severity: "success",
      target: "toast",
      title: options.title || "Success",
      message,
      duration_ms: options.duration_ms ?? 4000
    });
  },

  /**
   * Shows a warning toast
   * @param {string} message
   * @param {{ title?: string, duration_ms?: number }} [options]
   */
  warning(message, options = {}) {
    return notificationStore.dispatch({
      severity: "warning",
      target: "toast",
      title: options.title || "Warning",
      message,
      duration_ms: options.duration_ms ?? 6000
    });
  },

  /**
   * Shows an informative toast
   * @param {string} message
   * @param {{ title?: string, duration_ms?: number }} [options]
   */
  info(message, options = {}) {
    return notificationStore.dispatch({
      severity: "info",
      target: "toast",
      title: options.title || "Information",
      message,
      duration_ms: options.duration_ms ?? 4000
    });
  },

  /**
   * Pops up a modal error dialog (for critical failures, concurrency conflicts, or blocking alerts)
   * @param {any} err
   * @param {{ title?: string, actions?: Array<{ label: string, onClick: () => void, variant?: string }> }} [options]
   */
  modal(err, options = {}) {
    const norm = normalizeError(err);
    return notificationStore.dispatch({
      severity: "error",
      target: "popup",
      title: options.title || "Critical Error",
      message: norm.message,
      code: norm.code,
      details: norm.details,
      actions: options.actions || []
    });
  },

  /**
   * Dismisses all active toasts and closes the modal alert
   */
  clear() {
    notificationStore.toasts = [];
    notificationStore.modalAlert = null;
  }
};
