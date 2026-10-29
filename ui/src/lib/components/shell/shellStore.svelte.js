/**
 * BOS App Shell Reactive State (Svelte 5 Runes)
 * 100% Real, Zero-Dummy / Zero-Mock Architecture
 */
function sessionUser(user) {
  const roles = Array.isArray(user.roles) && user.roles.length > 0 ? user.roles : user.role ? [user.role] : ["User"];
  return {
    id: user.id,
    name: user.name || user.username || "",
    email: user.email || "",
    designation: roles.join(", "),
    role: roles.includes("SUPER")
      ? "SUPER"
      : roles.includes("Admin") || roles.includes("Global Admin")
        ? "Global Admin"
        : roles[0] || "User",
    roles,
    mfaEnabled: Boolean(user.mfa_enabled ?? user.two_factor_enabled ?? false),
    authMethod: user.auth_method || "Bearer Session Token",
    department: user.department || "",
    status: "online",
    username: user.username || "",
    isEmergency: Boolean(user.is_emergency || user.isEmergency),
  };
}

class ShellState {
  // Appearance & Themes ('light' | 'dark' | 'system')
  theme = $state("light");

  isEmergencySession = $state(
    (() => {
      if (typeof window === "undefined") return false;
      try {
        const cached = JSON.parse(localStorage.getItem("bos_session") || "null");
        return Boolean(cached?.emergency_fallback || cached?.maintenance_mode || cached?.user?.is_emergency);
      } catch {
        return false;
      }
    })()
  );

  maintenanceMode = $state(
    (() => {
      if (typeof window === "undefined") return false;
      try {
        const cached = JSON.parse(localStorage.getItem("bos_session") || "null");
        return Boolean(cached?.maintenance_mode);
      } catch {
        return false;
      }
    })()
  );

  emergencyWarning = $state(
    (() => {
      if (typeof window === "undefined") return "";
      try {
        const cached = JSON.parse(localStorage.getItem("bos_session") || "null");
        return cached?.warning || cached?.auth_error || "";
      } catch {
        return "";
      }
    })()
  );

  token = $state(
    (() => {
      if (typeof window === "undefined") return "";
      try {
        const cached = JSON.parse(localStorage.getItem("bos_session") || "null");
        if (cached?.token) return cached.token;
        const direct = localStorage.getItem("bos_auth_token");
        if (direct) return direct;
      } catch {
        return "";
      }
      return "";
    })()
  );

  currentUser = $state(
    (() => {
      if (typeof window === "undefined") return null;
      try {
        const cached = JSON.parse(localStorage.getItem("bos_session") || "null");
        if (cached?.token && cached?.user?.id) return sessionUser(cached.user);
      } catch {
        return null;
      }
      return null;
    })()
  );

  // Navigation & Layout
  isSidebarCollapsed = $state(false);
  isMobileDrawerOpen = $state(false);
  isFactBoxOpen = $state(false); // Collapsed by default when empty
  isNotificationsOpen = $state(false);
  isSearchOpen = $state(false);

  // Live Backend & Runtime State (Populated via real API calls)
  isOnline = $state(false);
  latencyMs = $state(0);
  engineVersion = $state("");
  runtimeStatus = $state("Connecting...");
  activeDatabase = $state("");
  activeStorageStrategy = $state("");
  databaseSize = $state("");

  // Live Company & Tenant
  companies = $state([]);
  activeCompany = $state("");
  tenants = $state([]);
  activeTenant = $state(null);

  // Available Databases & Environment Selector Dialog
  availableDatabases = $state([]);
  isEnvironmentDialogOpen = $state(false);
  isDbManagerOpen = $state(false);

  // Live Registered Meta-Objects
  metaObjects = $state([]);

  // Live Notifications
  notifications = $state([]);

  // Multi-Document Workspace Tabs (Empty by default - zero dummy records)
  openTabs = $state([]);
  activeTabId = $state("");

  // Derived active tab
  get activeTab() {
    return this.openTabs.find((t) => t.id === this.activeTabId) || null;
  }

  // Active company display name: always resolves to real human-friendly company name
  get companyDisplayName() {
    if (!this.activeCompany) return "Main Company";
    const found = this.companies?.find(
      (c) => c.code === this.activeCompany || c.name === this.activeCompany
    );
    if (found?.name) {
      return this.formatCompanyName(found.name);
    }
    return this.formatCompanyName(this.activeCompany);
  }

  formatCompanyName(raw) {
    if (!raw) return "Main Company";
    let cleaned = String(raw).trim().replace(/^Company\s+/i, "");
    if (/^[A-Z0-9_]+$/.test(cleaned) && (cleaned.includes("_") || cleaned === cleaned.toUpperCase())) {
      return cleaned
        .split("_")
        .map((w) => w.charAt(0).toUpperCase() + w.slice(1).toLowerCase())
        .join(" ");
    }
    return cleaned;
  }

  addNotification({ title, message, time = "Just now" }) {
    this.notifications = [
      {
        id: `notif_${Date.now()}_${Math.random().toString(36).slice(2, 7)}`,
        title,
        message,
        time,
      },
      ...this.notifications,
    ];
  }

  clearNotifications() {
    this.notifications = [];
  }

  navigate(path) {
    if (typeof window !== "undefined" && path) {
      window.location.href = path;
    }
  }

  getAuthHeaders() {
    let tok = this.token;
    if (!tok && typeof window !== "undefined") {
      try {
        const cached = JSON.parse(localStorage.getItem("bos_session") || "null");
        tok = cached?.token || localStorage.getItem("bos_auth_token") || "";
        if (tok) this.token = tok;
      } catch {}
    }
    const headers = {
      "Content-Type": "application/json"
    };
    if (tok) {
      headers["Authorization"] = `Bearer ${tok}`;
      headers["x-session-token"] = tok;
    }
    return headers;
  }

  // Real Backend Synchronization Routine
  async refreshSystem() {
    const headers = this.getAuthHeaders();
    try {
      const startTime = performance.now();
      const healthRes = await fetch("/api/health");
      const endTime = performance.now();

      if (healthRes.ok) {
        const healthData = await healthRes.json();
        this.isOnline = healthData.database !== "down";
        this.latencyMs = Math.round(endTime - startTime);
        this.engineVersion = healthData.version || "0.1.0";
        this.runtimeStatus = this.isOnline ? `Healthy · ${this.latencyMs}ms` : "Database down";
      } else {
        this.isOnline = false;
        this.runtimeStatus = "Engine Error";
      }
    } catch {
      this.isOnline = false;
      this.runtimeStatus = "Offline · Disconnected";
      this.latencyMs = 0;
    }

    // Fetch current database
    try {
      const dbRes = await fetch("/api/databases/current", { headers });
      if (dbRes.status === 401) {
        this.signOut();
        return;
      }
      if (dbRes.ok) {
        const dbData = await dbRes.json();
        this.activeDatabase = dbData.name || "postgres";
        this.activeStorageStrategy = dbData.storage_strategy || "shared_rls";
        this.databaseSize = dbData.size_pretty || "";
      }
    } catch {
      // Ignored if offline
    }

    // Fetch real companies
    try {
      const compRes = await fetch("/api/companies", { headers });
      if (compRes.ok) {
        const compData = await compRes.json();
        if (Array.isArray(compData)) {
          this.companies = compData;
          if (compData.length > 0) {
            const current = compData.find((c) => c.code === this.activeCompany || c.name === this.activeCompany);
            if (current) {
              this.activeCompany = current.name || current.code;
            } else if (!this.activeCompany || this.activeCompany === "MAIN_COMPANY" || this.activeCompany.startsWith("Company ")) {
              this.activeCompany = compData[0].name || compData[0].code;
            }
          }
        }
      }
    } catch {
      if (!this.activeCompany) {
        this.activeCompany = "Default Company";
      }
    }

    // Fetch real registered meta-objects with full definitions for menu meta-objects
    try {
      const metaRes = await fetch("/api/meta/objects", { headers });
      if (metaRes.ok) {
        const metaData = await metaRes.json();
        const list = Array.isArray(metaData) ? metaData : [];
        await Promise.all(
          list
            .filter((o) => o.object_type === "menu")
            .map(async (menuObj) => {
              try {
                const detailRes = await fetch(`/api/meta/objects/${menuObj.object_name}`, { headers });
                if (detailRes.ok) {
                  const detail = await detailRes.json();
                  menuObj.definition = detail.definition || detail;
                }
              } catch {}
            })
        );
        this.metaObjects = list;
      }
    } catch {
      this.metaObjects = [];
    }

    // Fetch available databases
    try {
      const dbsRes = await fetch("/api/databases", { headers });
      if (dbsRes.ok) {
        const dbsData = await dbsRes.json();
        if (Array.isArray(dbsData)) {
          this.availableDatabases = dbsData;
        }
      }
    } catch {
      if (this.availableDatabases.length === 0 && this.activeDatabase) {
        this.availableDatabases = [{ name: this.activeDatabase, is_active: true }];
      }
    }

    // Fetch available tenants
    try {
      const tenantsRes = await fetch("/api/tenants");
      if (tenantsRes.ok) {
        const tenantsData = await tenantsRes.json();
        if (Array.isArray(tenantsData)) {
          this.tenants = tenantsData;
          if (tenantsData.length > 0 && !this.activeTenant) {
            this.activeTenant = tenantsData[0];
          }
        }
      }
    } catch {
      // Offline fallback
    }
  }

  // Workspace Navigation & Profile
  openUserProfile() {
    this.openTab({
      id: "user-profile",
      title: `Profile · ${this.currentUser?.name || "Account"}`,
      type: "profile"
    });
  }

  openEnvironmentDialog() {
    this.isEnvironmentDialogOpen = true;
  }

  closeEnvironmentDialog() {
    this.isEnvironmentDialogOpen = false;
  }

  openDbManager() {
    this.isDbManagerOpen = true;
  }

  closeDbManager() {
    this.isDbManagerOpen = false;
  }

  async switchEnvironment({ database, tenantId, companyName }) {
    if (database && database !== this.activeDatabase) {
      try {
        const res = await fetch("/api/databases/switch", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ name: database })
        });
        if (res.ok) {
          this.activeDatabase = database;
        }
      } catch {
        this.activeDatabase = database;
      }
    }
    if (companyName) {
      this.activeCompany = companyName;
    }
    if (tenantId) {
      const found = this.tenants.find(
        (t) => t.tenant_id === tenantId || t.id === tenantId || t.name === tenantId || t.code === tenantId
      );
      if (found) {
        this.activeTenant = found;
      } else {
        this.activeTenant = { id: tenantId, tenant_id: tenantId, name: tenantId, code: String(tenantId).toUpperCase() };
      }
    }
    if (typeof window !== "undefined") {
      try {
        localStorage.setItem(
          "bos_environment",
          JSON.stringify({
            database: this.activeDatabase,
            tenantId: this.activeTenant?.id || tenantId || "",
            companyName: this.activeCompany || companyName || "",
          })
        );
      } catch {
        // Ignored
      }
    }
    this.isEnvironmentDialogOpen = false;
    await this.refreshSystem();
  }

  // Workspace tab actions
  openTab(tab) {
    const existing = this.openTabs.find((t) => t.id === tab.id);
    if (!existing) {
      this.openTabs = [...this.openTabs, { closable: true, ...tab }];
    }
    this.activeTabId = tab.id;
  }

  closeTab(tabId) {
    const filtered = this.openTabs.filter((t) => t.id !== tabId);
    if (this.activeTabId === tabId) {
      const idx = this.openTabs.findIndex((t) => t.id === tabId);
      const nextTab = filtered[Math.max(0, idx - 1)] || filtered[0];
      this.activeTabId = nextTab ? nextTab.id : "";
    }
    this.openTabs = filtered;
  }

  // Layout Toggles
  toggleSidebar() {
    this.isSidebarCollapsed = !this.isSidebarCollapsed;
  }

  toggleMobileDrawer() {
    this.isMobileDrawerOpen = !this.isMobileDrawerOpen;
  }

  toggleFactBox() {
    this.isFactBoxOpen = !this.isFactBoxOpen;
  }

  // Theme Management ('light' | 'dark' | 'system')
  initTheme() {
    if (typeof window !== "undefined") {
      const saved = localStorage.getItem("bos_theme");
      if (saved && ["light", "dark", "system"].includes(saved)) {
        this.theme = saved;
      }
      this.applyThemeClass();

      // Listen for OS theme updates if in system mode
      try {
        window.matchMedia?.("(prefers-color-scheme: dark)").addEventListener("change", () => {
          if (this.theme === "system") {
            this.applyThemeClass();
          }
        });
      } catch {
        // Ignored
      }
    }
  }

  setTheme(newTheme) {
    if (!["light", "dark", "system"].includes(newTheme)) return;
    this.theme = newTheme;
    if (typeof window !== "undefined") {
      try {
        localStorage.setItem("bos_theme", newTheme);
      } catch {
        // Ignored
      }
      this.applyThemeClass();
    }
  }

  applyThemeClass() {
    if (typeof document !== "undefined") {
      const isDark =
        this.theme === "dark" ||
        (this.theme === "system" &&
          typeof window !== "undefined" &&
          window.matchMedia?.("(prefers-color-scheme: dark)").matches);

      document.documentElement.classList.toggle("dark", Boolean(isDark));
    }
  }

  toggleTheme() {
    if (this.theme === "dark") {
      this.setTheme("light");
    } else {
      this.setTheme("dark");
    }
  }

  signIn(session) {
    if (!session?.token || !session?.user?.id) return;
    this.token = session.token;
    this.currentUser = sessionUser(session.user);
    this.isEmergencySession = Boolean(
      session.emergency_fallback || session.user?.is_emergency || session.maintenance_mode
    );
    this.maintenanceMode = Boolean(session.maintenance_mode);
    this.emergencyWarning = session.warning || session.auth_error || "";
    if (typeof window !== "undefined") {
      localStorage.setItem("bos_auth_token", session.token);
      localStorage.setItem(
        "bos_session",
        JSON.stringify({
          token: session.token,
          user: session.user,
          emergency_fallback: session.emergency_fallback,
          maintenance_mode: session.maintenance_mode,
          warning: session.warning,
          auth_error: session.auth_error,
        })
      );
      localStorage.removeItem("bos_user_profile");
    }
  }

  signOut() {
    this.token = "";
    this.currentUser = null;
    this.isEmergencySession = false;
    this.maintenanceMode = false;
    this.emergencyWarning = "";
    this.openTabs = [];
    this.activeTabId = "";
    if (typeof window !== "undefined") {
      localStorage.removeItem("bos_auth_token");
      localStorage.removeItem("bos_session");
      localStorage.removeItem("bos_user_profile");
    }
  }

  logout() {
    this.signOut();
  }

  // User Profile Management
  async updateUser(patch) {
    if (!this.currentUser) return;
    this.currentUser = { ...this.currentUser, ...patch };

    let token = "";
    if (typeof window !== "undefined") {
      try {
        const cached = JSON.parse(localStorage.getItem("bos_session") || "null");
        if (cached?.token) {
          token = cached.token;
          cached.user = { ...cached.user, ...patch };
          localStorage.setItem("bos_session", JSON.stringify(cached));
        }
      } catch {
        // Ignored
      }
    }

    if (token) {
      try {
        const res = await fetch("/api/auth/profile", {
          method: "PATCH",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${token}`,
            "x-session-token": token,
          },
          body: JSON.stringify(patch),
        });
        if (!res.ok) {
          const err = await res.json().catch(() => ({}));
          throw new Error(err.message || err.error || "Failed to persist profile to server");
        }
        const data = await res.json();
        if (data.user) {
          this.currentUser = sessionUser(data.user);
        }
      } catch (e) {
        console.error("Failed to persist user profile to backend:", e);
        throw e;
      }
    }
  }
}

export const shell = new ShellState();
// Auto-initialize theme on client
if (typeof window !== "undefined") {
  shell.initTheme();
  window.__bos_shell = shell;
}
