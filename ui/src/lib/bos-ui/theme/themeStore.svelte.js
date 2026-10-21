/**
 * BOS Global Personalization & Theming Store
 * Single saved setting for density, surface variant, corner radius, color theme, and work date.
 * Class names for those choices live in surface.js and are applied by every bos-ui control.
 */

const STORAGE_KEY = "bos_user_preference";

class ThemeStore {
  // Svelte 5 Runes state
  density = $state("md"); // 'sm' (compact 28px ERP) | 'md' (36px) | 'lg' (44px touch)
  variant = $state("outline"); // 'outline' | 'filled' | 'underline' | 'ghost'
  rounded = $state("md"); // 'none' (0px sharp) | 'sm' (3px) | 'md' (6px) | 'pill'
  theme = $state("system"); // 'light' | 'dark' | 'system'
  workDate = $state(new Date().toISOString().split("T")[0]); // Simulated posting date
  dateFormat = $state("YYYY-MM-DD");

  constructor() {
    if (typeof window !== "undefined") {
      try {
        const saved = localStorage.getItem(STORAGE_KEY);
        if (saved) {
          const parsed = JSON.parse(saved);
          if (parsed.density) this.density = parsed.density;
          if (parsed.variant) this.variant = parsed.variant;
          if (parsed.rounded) this.rounded = parsed.rounded;
          if (parsed.theme) this.theme = parsed.theme;
          if (parsed.workDate) this.workDate = parsed.workDate;
          if (parsed.dateFormat) this.dateFormat = parsed.dateFormat;
        }
      } catch {
        // Safe fallback
      }
    }
  }

  save() {
    if (typeof window !== "undefined") {
      try {
        localStorage.setItem(
          STORAGE_KEY,
          JSON.stringify({
            density: this.density,
            variant: this.variant,
            rounded: this.rounded,
            theme: this.theme,
            workDate: this.workDate,
            dateFormat: this.dateFormat
          })
        );
      } catch {
        // Safe fallback
      }
    }
  }

  setDensity(d) {
    this.density = d;
    this.save();
  }

  setVariant(v) {
    this.variant = v;
    this.save();
  }

  setRounded(r) {
    this.rounded = r;
    this.save();
  }

  setTheme(t) {
    this.theme = t;
    this.save();
  }

  setWorkDate(w) {
    this.workDate = w;
    this.save();
  }

  resetDefaults() {
    this.density = "md";
    this.variant = "outline";
    this.rounded = "md";
    this.theme = "system";
    this.workDate = new Date().toISOString().split("T")[0];
    this.save();
  }
}

export const themeStore = new ThemeStore();
