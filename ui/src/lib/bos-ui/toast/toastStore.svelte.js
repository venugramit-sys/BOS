class ToastStore {
  items = $state([]);

  /**
   * @param {{ tone?: 'info' | 'success' | 'warning' | 'danger', title?: string, description?: string, timeout?: number }} toast
   * @returns {string}
   */
  push(toast) {
    const id = crypto.randomUUID();
    this.items = [
      ...this.items,
      {
        id,
        tone: toast.tone || "info",
        title: toast.title || "",
        description: toast.description || "",
        timeout: toast.timeout ?? 4000
      }
    ];
    return id;
  }

  dismiss(id) {
    this.items = this.items.filter((item) => item.id !== id);
  }
}

export const toastStore = new ToastStore();
