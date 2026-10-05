import { useLayoutEffect, useState } from "react";

export type Theme = "system" | "dark" | "light";
export const THEME_STORAGE_KEY = "ets2-workshop.theme";

function storedTheme(): Theme {
  try {
    const saved = localStorage.getItem(THEME_STORAGE_KEY);
    return saved === "light" || saved === "dark" ? saved : "system";
  } catch {
    return "system";
  }
}

export function applyStoredTheme() {
  const theme = storedTheme();
  document.documentElement.dataset.theme =
    theme === "system"
      ? window.matchMedia("(prefers-color-scheme: dark)").matches
        ? "dark"
        : "light"
      : theme;
}

export function useTheme() {
  const [theme, setTheme] = useState<Theme>(storedTheme);
  useLayoutEffect(() => {
    const systemTheme = window.matchMedia("(prefers-color-scheme: dark)");
    const applyTheme = () => {
      document.documentElement.dataset.theme =
        theme === "system" ? (systemTheme.matches ? "dark" : "light") : theme;
    };
    applyTheme();
    try {
      localStorage.setItem(THEME_STORAGE_KEY, theme);
    } catch {
      // Theme switching still works when browser storage is unavailable.
    }
    if (theme === "system") {
      systemTheme.addEventListener("change", applyTheme);
      return () => systemTheme.removeEventListener("change", applyTheme);
    }
  }, [theme]);
  return { theme, setTheme };
}
