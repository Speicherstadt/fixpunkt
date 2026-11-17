/** Hell, dunkel oder wie das Gerät – gespeichert auf diesem Gerät (siehe public/theme.js) */
export type Theme = "light" | "dark" | "system";

const KEY = "fixpunkt.theme";

export const storedTheme = (): Theme => {
  try {
    const v = localStorage.getItem(KEY);
    return v === "light" || v === "dark" ? v : "system";
  } catch {
    return "system";
  }
};

export const isDark = (theme: Theme, deviceDark: boolean): boolean =>
  theme === "dark" || (theme === "system" && deviceDark);

export const applyTheme = (theme: Theme) => {
  try {
    if (theme === "system") localStorage.removeItem(KEY);
    else localStorage.setItem(KEY, theme);
  } catch {
    // Speicher verweigert: gilt nur bis zum Neuladen
  }
  const dark = isDark(theme, matchMedia("(prefers-color-scheme: dark)").matches);
  document.documentElement.classList.toggle("dark", dark);
  document.documentElement.classList.toggle("light", !dark);
};
