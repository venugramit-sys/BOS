// @ts-check

/**
 * @typedef {'info' | 'success' | 'warning' | 'error'} NotificationSeverity
 * @typedef {'toast' | 'popup' | 'infield' | 'banner'} NotificationTarget
 * 
 * @typedef {Object} NotificationItem
 * @property {string} id
 * @property {NotificationSeverity} severity
 * @property {NotificationTarget} target
 * @property {string} [title]
 * @property {string} message
 * @property {string} [field]
 * @property {string} [code]
 * @property {string} [details]
 * @property {number} [duration_ms]
 * @property {string} timestamp
 * @property {boolean} [read]
 * @property {Array<any>} [actions]
 */

class NotificationStore {
  /** @type {NotificationItem[]} */
  toasts = $state([]);

  /** @type {NotificationItem | null} */
  modalAlert = $state(null);

  /** @type {Record<string, string>} */
  fieldErrors = $state({});

  /** @type {NotificationItem[]} */
  history = $state([]);

  /** @type {boolean} */
  isSseConnected = $state(false);

  /** @type {EventSource | null} */
  #eventSource = null;

  /** @type {any} */
  #reconnectTimer = null;

  get unreadCount() {
    return this.history.filter((n) => !n.read).length;
  }

  /**
   * Dispatches a unified notification into appropriate channels
   * @param {Partial<NotificationItem> & { message: string }} notif
   */
  dispatch(notif) {
    const item = {
      id: notif.id || `notif_${Date.now()}_${Math.random().toString(36).slice(2, 7)}`,
      severity: notif.severity || "info",
      target: notif.target || "toast",
      title: notif.title || "",
      message: notif.message,
      field: notif.field,
      code: notif.code,
      details: notif.details,
      duration_ms: notif.duration_ms ?? (notif.severity === "error" ? 8000 : 5000),
      timestamp: notif.timestamp || new Date().toISOString(),
      read: false,
      actions: notif.actions || [],
    };

    // 1. Add to historical log for the Notification Drawer
    this.history = [item, ...this.history.slice(0, 99)];

    // 2. Deliver according to target
    switch (item.target) {
      case "popup":
        this.modalAlert = item;
        break;
      case "infield":
        if (item.field) {
          this.fieldErrors = {
            ...this.fieldErrors,
            [item.field]: item.message,
          };
        }
        // Also show a toast if critical
        if (item.severity === "error") {
          this.#addToast(item);
        }
        break;
      case "banner":
      case "toast":
      default:
        this.#addToast(item);
        break;
    }

    return item.id;
  }

  /**
   * @param {NotificationItem} item
   */
  #addToast(item) {
    this.toasts = [...this.toasts, item];

    if (item.duration_ms && item.duration_ms > 0) {
      setTimeout(() => {
        this.dismissToast(item.id);
      }, item.duration_ms);
    }
  }

  /**
   * Helper to display a toast
   * @param {{ message: string, severity?: NotificationSeverity, title?: string, details?: string, code?: string, duration_ms?: number }} param0
   */
  showToast({ message, severity = "info", title = "", details = "", code = "", duration_ms }) {
    return this.dispatch({
      message,
      severity,
      title,
      details,
      code,
      target: "toast",
      duration_ms,
    });
  }

  /**
   * Helper to display a popup modal alert
   * @param {{ message: string, severity?: NotificationSeverity, title?: string, details?: string, code?: string, actions?: any[] }} param0
   */
  showPopup({ message, severity = "error", title = "Alert", details = "", code = "", actions = [] }) {
    return this.dispatch({
      message,
      severity,
      title,
      details,
      code,
      target: "popup",
      actions,
    });
  }

  closePopup() {
    this.modalAlert = null;
  }

  dismissToast(id) {
    this.toasts = this.toasts.filter((t) => t.id !== id);
  }

  setFieldError(field, message) {
    this.fieldErrors = {
      ...this.fieldErrors,
      [field]: message,
    };
  }

  clearFieldError(field) {
    if (this.fieldErrors[field]) {
      const next = { ...this.fieldErrors };
      delete next[field];
      this.fieldErrors = next;
    }
  }

  clearAllFieldErrors() {
    this.fieldErrors = {};
  }

  markAllRead() {
    this.history = this.history.map((n) => ({ ...n, read: true }));
  }

  clearHistory() {
    this.history = [];
  }

  /**
   * Inspects HTTP response and payload to automatically display notifications and field errors
   * @param {Response} response
   * @param {any} data
   */
  handleApiResponse(response, data = {}) {
    let hasNotified = false;

    // 1. Parse 'x-bos-notifications' header if present
    const headerNotifs = response.headers.get("x-bos-notifications");
    if (headerNotifs) {
      try {
        const parsed = JSON.parse(headerNotifs);
        if (Array.isArray(parsed)) {
          for (const notif of parsed) {
            this.dispatch(notif);
            hasNotified = true;
          }
        }
      } catch (err) {
        console.warn("[notificationStore] Failed to parse x-bos-notifications header", err);
      }
    }

    // 2. Parse response body notifications array
    if (data && Array.isArray(data.notifications)) {
      for (const notif of data.notifications) {
        this.dispatch(notif);
        hasNotified = true;
      }
    }

    // 3. Parse in-field validation errors
    if (data && data.field_errors && typeof data.field_errors === "object") {
      this.fieldErrors = {
        ...this.fieldErrors,
        ...data.field_errors,
      };
      hasNotified = true;
    }

    // 4. Fallback if request failed and no notification was surfaced
    if (!response.ok && !hasNotified) {
      const errorMsg = data?.message || data?.error || `Request failed with status ${response.status}`;
      this.dispatch({
        severity: "error",
        target: response.status >= 500 ? "popup" : "toast",
        title: response.status >= 500 ? "Server Error" : "Validation Error",
        message: errorMsg,
        code: data?.code || `HTTP_${response.status}`,
        details: typeof data === "object" ? JSON.stringify(data, null, 2) : String(data),
      });
    }

    return !hasNotified && !response.ok;
  }

  /**
   * Initializes real-time SSE notification stream
   * @param {string} [baseUrl]
   */
  initSse(baseUrl = "") {
    if (typeof window === "undefined") return;
    if (this.#eventSource) {
      this.#eventSource.close();
      this.#eventSource = null;
    }

    const sseUrl = `${baseUrl}/api/notifications/stream`;

    try {
      const es = new EventSource(sseUrl);
      this.#eventSource = es;

      es.onopen = () => {
        this.isSseConnected = true;
        if (this.#reconnectTimer) {
          clearTimeout(this.#reconnectTimer);
          this.#reconnectTimer = null;
        }
      };

      es.onmessage = (event) => {
        if (!event.data || event.data === "ping") return;
        try {
          const payload = JSON.parse(event.data);
          this.dispatch(payload);
        } catch (err) {
          console.warn("[notificationStore] SSE parsing error:", err, event.data);
        }
      };

      es.onerror = () => {
        this.isSseConnected = false;
        es.close();
        this.#eventSource = null;

        // Auto-reconnect with backoff
        if (!this.#reconnectTimer) {
          this.#reconnectTimer = setTimeout(() => {
            this.#reconnectTimer = null;
            this.initSse(baseUrl);
          }, 5000);
        }
      };
    } catch (err) {
      console.warn("[notificationStore] Unable to create EventSource:", err);
    }
  }

  destroy() {
    if (this.#eventSource) {
      this.#eventSource.close();
      this.#eventSource = null;
    }
    if (this.#reconnectTimer) {
      clearTimeout(this.#reconnectTimer);
      this.#reconnectTimer = null;
    }
  }
}

export const notificationStore = new NotificationStore();
