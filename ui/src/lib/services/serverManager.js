/**
 * Server Connection & Recent Servers Manager
 * Persists and manages the last 3 successfully connected servers for 1-click connect.
 */

const STORAGE_KEY = "bos_recent_servers";
const ACTIVE_SERVER_KEY = "bos_active_server";

export const DEFAULT_LOCAL_SERVER = {
  id: "local_engine",
  name: "Local PostgreSQL",
  host: "127.0.0.1:5432",
  url: "postgresql://postgres:postgres@localhost:5432/postgres",
  provider: "local",
  lastConnected: "Default Engine",
  timestamp: Date.now()
};

export const CLOUD_PRESETS = [
  {
    id: "supabase",
    name: "Supabase",
    provider: "supabase",
    placeholder: "postgresql://postgres:[PASSWORD]@db.[REF].supabase.co:5432/postgres?sslmode=require"
  },
  {
    id: "neon",
    name: "Neon",
    provider: "neon",
    placeholder: "postgresql://[USER]:[PASSWORD]@[ENDPOINT].neon.tech/neondb?sslmode=require"
  },
  {
    id: "aws",
    name: "AWS RDS",
    provider: "aws",
    placeholder: "postgresql://postgres:[PASSWORD]@[ENDPOINT].rds.amazonaws.com:5432/postgres?sslmode=require"
  }
];

export function detectProvider(url = "") {
  const lower = url.toLowerCase();
  if (lower.includes("supabase.co")) return "supabase";
  if (lower.includes("neon.tech")) return "neon";
  if (lower.includes("rds.amazonaws.com")) return "aws";
  if (lower.includes("localhost") || lower.includes("127.0.0.1")) return "local";
  return "custom";
}

export function extractHostFromUrl(url = "") {
  try {
    if (!url.startsWith("postgres://") && !url.startsWith("postgresql://")) {
      return url.split("/")[0] || "127.0.0.1:5432";
    }
    const clean = url.replace(/^postgres(ql)?:\/\//, "");
    const atIndex = clean.indexOf("@");
    const hostPart = atIndex !== -1 ? clean.substring(atIndex + 1) : clean;
    const slashIndex = hostPart.indexOf("/");
    const qIndex = hostPart.indexOf("?");
    let end = hostPart.length;
    if (slashIndex !== -1) end = Math.min(end, slashIndex);
    if (qIndex !== -1) end = Math.min(end, qIndex);
    return hostPart.substring(0, end) || "127.0.0.1:5432";
  } catch {
    return "127.0.0.1:5432";
  }
}

export function formatRelativeTime(timestamp) {
  if (!timestamp) return "Recent";
  const diffSec = Math.floor((Date.now() - timestamp) / 1000);
  if (diffSec < 60) return "Just now";
  const diffMin = Math.floor(diffSec / 60);
  if (diffMin < 60) return `${diffMin}m ago`;
  const diffHours = Math.floor(diffMin / 60);
  if (diffHours < 24) return `${diffHours}h ago`;
  const diffDays = Math.floor(diffHours / 24);
  return `${diffDays}d ago`;
}

/**
 * Returns the last 3 successfully connected servers.
 * @param {boolean} includeLocal Whether to include local server (only if detected)
 */
export function getRecentServers(includeLocal = true) {
  if (typeof window === "undefined") return includeLocal ? [DEFAULT_LOCAL_SERVER] : [];
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return includeLocal ? [DEFAULT_LOCAL_SERVER] : [];
    const parsed = JSON.parse(raw);
    if (Array.isArray(parsed) && parsed.length > 0) {
      let filtered = parsed;
      if (!includeLocal) {
        filtered = parsed.filter(
          (s) => s.provider !== "local" && !s.host?.includes("127.0.0.1") && !s.host?.includes("localhost")
        );
      }
      return filtered.slice(0, 3).map((srv) => ({
        ...srv,
        lastConnected: formatRelativeTime(srv.timestamp)
      }));
    }
  } catch {
    // Fallback
  }
  return includeLocal ? [DEFAULT_LOCAL_SERVER] : [];
}

/**
 * Persists a successfully connected server and keeps the last 3.
 */
export function saveRecentServer({ name, url, host, provider }) {
  if (typeof window === "undefined") return;
  try {
    const finalHost = host || extractHostFromUrl(url);
    const finalProvider = provider || detectProvider(url);
    const finalName =
      name ||
      (finalProvider === "local"
        ? "Local PostgreSQL"
        : finalProvider === "supabase"
        ? "Supabase Cloud"
        : finalProvider === "neon"
        ? "Neon Cloud"
        : finalProvider === "aws"
        ? "AWS RDS"
        : finalHost);

    const currentList = getRecentServers(true);
    // Deduplicate by host or url
    const filtered = currentList.filter(
      (s) => s.url !== url && s.host !== finalHost
    );

    const newServer = {
      id: `srv_${Date.now()}_${Math.random().toString(36).slice(2, 6)}`,
      name: finalName,
      url: url || `postgresql://postgres:postgres@${finalHost}/postgres`,
      host: finalHost,
      provider: finalProvider,
      lastConnected: "Just now",
      timestamp: Date.now()
    };

    const updated = [newServer, ...filtered].slice(0, 3);
    localStorage.setItem(STORAGE_KEY, JSON.stringify(updated));
    localStorage.setItem(ACTIVE_SERVER_KEY, JSON.stringify(newServer));
    return updated;
  } catch {
    return [DEFAULT_LOCAL_SERVER];
  }
}

/**
 * Returns the currently active server.
 * @param {boolean} includeLocal Whether to consider local server active if none other exists
 */
export function getActiveServer(includeLocal = true) {
  if (typeof window === "undefined") return includeLocal ? DEFAULT_LOCAL_SERVER : null;
  try {
    const raw = localStorage.getItem(ACTIVE_SERVER_KEY);
    if (raw) {
      const parsed = JSON.parse(raw);
      if (includeLocal || (parsed.provider !== "local" && !parsed.host?.includes("127.0.0.1") && !parsed.host?.includes("localhost"))) {
        return parsed;
      }
    }
  } catch {}
  return getRecentServers(includeLocal)[0] || (includeLocal ? DEFAULT_LOCAL_SERVER : null);
}
