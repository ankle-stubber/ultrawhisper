import { create } from "zustand";
import { persist } from "zustand/middleware";

export type ThemeName = "terminal" | "green" | "amber" | "custom";

export interface CustomColors {
  primary: string;
  surface: string;
  text: string;
  border: string;
}

interface ThemeStore {
  currentTheme: ThemeName;
  customColors: CustomColors;

  // Actions
  setTheme: (theme: ThemeName) => void;
  updateCustomColor: (key: keyof CustomColors, value: string) => void;
  resetCustomColors: () => void;
  getThemeClassName: () => string;
}

// Default custom color values (Terminal theme as base)
const DEFAULT_CUSTOM_COLORS: CustomColors = {
  primary: "#1ed7d7",
  surface: "#0a0e1a",
  text: "#e1e8f0",
  border: "#1a2332",
};

// Theme presets for reference/preview
export const THEME_PRESETS = {
  terminal: {
    name: "Terminal",
    description: "Cyan accent, terminal aesthetic",
    primary: "#1ed7d7",
  },
  green: {
    name: "Green",
    description: "Brand green, classic look",
    primary: "#17951e",
  },
  amber: {
    name: "Amber",
    description: "Warm amber accent",
    primary: "#fbbf24",
  },
  custom: {
    name: "Custom",
    description: "Your personalized colors",
    primary: "#1ed7d7", // Placeholder
  },
} as const;

export const useThemeStore = create<ThemeStore>()(
  persist(
    (set, get) => ({
      currentTheme: "terminal",
      customColors: DEFAULT_CUSTOM_COLORS,

      setTheme: (theme) => {
        set({ currentTheme: theme });
        // Apply custom CSS variables if custom theme
        if (theme === "custom") {
          applyCustomTheme(get().customColors);
        } else {
          removeCustomTheme();
        }
      },

      updateCustomColor: (key, value) => {
        set((state) => ({
          customColors: {
            ...state.customColors,
            [key]: value,
          },
        }));
        // If currently on custom theme, apply immediately
        if (get().currentTheme === "custom") {
          applyCustomTheme(get().customColors);
        }
      },

      resetCustomColors: () => {
        set({ customColors: DEFAULT_CUSTOM_COLORS });
        if (get().currentTheme === "custom") {
          applyCustomTheme(DEFAULT_CUSTOM_COLORS);
        }
      },

      getThemeClassName: () => {
        return `theme-${get().currentTheme}`;
      },
    }),
    {
      name: "uw-theme-config", // localStorage key
      partialize: (state) => ({
        currentTheme: state.currentTheme,
        customColors: state.customColors,
      }),
    }
  )
);

// Helper function to apply custom theme CSS variables
function applyCustomTheme(colors: CustomColors) {
  const root = document.documentElement;

  // Calculate derived colors from primary
  const primary = colors.primary;
  const primaryRgb = hexToRgb(primary);

  if (primaryRgb) {
    // Set CSS custom properties
    root.style.setProperty("--uw-primary", primary);
    root.style.setProperty("--uw-primary-hover", adjustBrightness(primary, -10));
    root.style.setProperty("--uw-primary-dim", `rgba(${primaryRgb.r}, ${primaryRgb.g}, ${primaryRgb.b}, 0.1)`);
    root.style.setProperty("--uw-primary-border", `rgba(${primaryRgb.r}, ${primaryRgb.g}, ${primaryRgb.b}, 0.3)`);
  }

  root.style.setProperty("--uw-surface", colors.surface);
  root.style.setProperty("--uw-text", colors.text);
  root.style.setProperty("--uw-border", colors.border);
}

// Helper function to remove custom theme variables
function removeCustomTheme() {
  const root = document.documentElement;
  root.style.removeProperty("--uw-primary");
  root.style.removeProperty("--uw-primary-hover");
  root.style.removeProperty("--uw-primary-dim");
  root.style.removeProperty("--uw-primary-border");
  root.style.removeProperty("--uw-surface");
  root.style.removeProperty("--uw-text");
  root.style.removeProperty("--uw-border");
}

// Helper: Convert hex to RGB
function hexToRgb(hex: string): { r: number; g: number; b: number } | null {
  const result = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
  return result
    ? {
        r: parseInt(result[1], 16),
        g: parseInt(result[2], 16),
        b: parseInt(result[3], 16),
      }
    : null;
}

// Helper: Adjust color brightness
function adjustBrightness(hex: string, percent: number): string {
  const rgb = hexToRgb(hex);
  if (!rgb) return hex;

  const adjust = (value: number) => {
    const adjusted = value + (value * percent) / 100;
    return Math.max(0, Math.min(255, Math.round(adjusted)));
  };

  const r = adjust(rgb.r);
  const g = adjust(rgb.g);
  const b = adjust(rgb.b);

  return `#${((1 << 24) + (r << 16) + (g << 8) + b).toString(16).slice(1)}`;
}

// Initialize theme on app start
export function initializeTheme() {
  const theme = useThemeStore.getState().currentTheme;
  if (theme === "custom") {
    const colors = useThemeStore.getState().customColors;
    applyCustomTheme(colors);
  }
}
