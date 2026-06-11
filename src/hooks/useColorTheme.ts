import { getCurrentWindow } from "@tauri-apps/api/window";
import { atom, useAtom } from "jotai";
import { useEffect, useState } from "react";
import { darkClasses } from "@/consts/style";
import type { Theme } from "@/rspc/bindings";

const defaultTheme = ["dark", "light"];

export const currentThemeAtom = atom<Exclude<Theme, "system"> | null>(null);

export const useColorTheme = (theme: Theme) => {
  const [, setCurrentTheme] = useAtom(currentThemeAtom);
  const [systemTheme, setSystemTheme] = useState<"dark" | "light">("light");

  useEffect(() => {
    getCurrentWindow()
      .theme()
      .then((t) => setSystemTheme(t === "dark" ? "dark" : "light"));

    const cleanup = getCurrentWindow().onThemeChanged(({ payload: theme }) => {
      setSystemTheme(theme === "dark" ? "dark" : "light");
    });

    return () => {
      cleanup.then((unlisten) => unlisten());
    };
  }, []);

  useEffect(() => {
    const applyTheme = (theme: Exclude<Theme, "system">) => {
      document.documentElement.classList.add(theme);
      setCurrentTheme(theme);
    };

    document.documentElement.classList.remove(...defaultTheme);
    document.documentElement.dataset.theme = "";

    // Apply System Theme
    if (theme === "system") {
      applyTheme(systemTheme);
      return;
    }

    // Apply Dark / Light Theme
    if (defaultTheme.includes(theme)) {
      applyTheme(theme);
      return;
    }

    // Apply Other Theme
    if (darkClasses.includes(theme)) {
      applyTheme("dark");
    }

    document.documentElement.dataset.theme = theme;
  }, [theme, systemTheme, setCurrentTheme]);
};
