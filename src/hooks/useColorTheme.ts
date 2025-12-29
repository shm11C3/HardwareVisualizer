import { getCurrentWindow } from "@tauri-apps/api/window";
import { atom, useAtom } from "jotai";
import { useCallback, useEffect, useState } from "react";
import { darkClasses } from "@/consts/style";
import type { Theme } from "@/rspc/bindings";

const defaultTheme = ["dark", "light"];

export const currentThemeAtom = atom<Exclude<Theme, "system"> | null>(null);

export const useColorTheme = (theme: Theme) => {
  const [, setCurrentTheme] = useAtom(currentThemeAtom);
  const [systemTheme, setSystemTheme] = useState<"dark" | "light">("light");

  const listenTheme = useCallback(async () => {
    const unlisten = await getCurrentWindow().onThemeChanged(
      ({ payload: theme }) => {
        setSystemTheme(theme === "dark" ? "dark" : "light");
      },
    );

    return () => {
      unlisten();
    };
  }, []);

  useEffect(() => {
    getCurrentWindow()
      .theme()
      .then((t) => setSystemTheme(t === "dark" ? "dark" : "light"));

    const cleanup = listenTheme();
    return () => {
      cleanup.then((f) => f());
    };
  }, [listenTheme]);

  const applyTheme = useCallback(
    (theme: Exclude<Theme, "system">) => {
      document.documentElement.classList.add(theme);
      setCurrentTheme(theme);
    },
    [setCurrentTheme],
  );

  useEffect(() => {
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
  }, [theme, systemTheme, applyTheme]);
};
