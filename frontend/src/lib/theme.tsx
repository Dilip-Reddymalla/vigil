import { createContext, useContext, useEffect, useState, type ReactNode } from "react";
import { getSetting, setSetting } from "@/lib/api";

type Theme = "light" | "dark" | "system";

interface ThemeContextValue {
  theme: Theme;
  setTheme: (t: Theme) => void;
}

const STORAGE_KEY = "vigil.theme";

const ThemeContext = createContext<ThemeContextValue>({ theme: "system", setTheme: () => {} });

function applyTheme(theme: Theme) {
  const root = document.documentElement;
  root.classList.remove("light", "dark");
  if (theme === "system") {
    const dark = window.matchMedia("(prefers-color-scheme: dark)").matches;
    root.classList.add(dark ? "dark" : "light");
  } else {
    root.classList.add(theme);
  }
}

function initFromStorage(): Theme {
  try {
    const v = localStorage.getItem(STORAGE_KEY);
    if (v === "light" || v === "dark") return v;
  } catch {
    /* ignore */
  }
  return "system";
}

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [theme, setThemeState] = useState<Theme>(initFromStorage);

  // Apply + persist locally + write through to the service settings.
  useEffect(() => {
    applyTheme(theme);
    try {
      localStorage.setItem(STORAGE_KEY, theme);
    } catch {
      /* ignore */
    }
    // Best-effort: persist on the service too, so it follows across devices/sessions.
    setSetting("theme", theme, "string").catch(() => {});
  }, [theme]);

  // Prefer the service-level theme on first load (device-wide preference).
  useEffect(() => {
    let alive = true;
    getSetting("theme").then((v) => {
      if (!alive) return;
      if (v === "light" || v === "dark" || v === "system") {
        setThemeState(v);
      }
    });
    return () => {
      alive = false;
    };
  }, []);

  const setTheme = (t: Theme) => setThemeState(t);

  return <ThemeContext.Provider value={{ theme, setTheme }}>{children}</ThemeContext.Provider>;
}

export function useTheme() {
  return useContext(ThemeContext);
}
