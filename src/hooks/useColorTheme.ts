import { getCurrentWindow } from "@tauri-apps/api/window";
import { useCallback, useEffect, useState } from "react";
import { darkClasses } from "@/consts/style";
import type { Theme } from "@/rspc/bindings";

const defaultTheme = ["dark", "light"];

export const useColorTheme = (theme: Theme) => {
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
    return cleanup;
  }, [listenTheme]);

  useEffect(() => {
    document.documentElement.classList.remove(...defaultTheme);
    document.documentElement.dataset.theme = "";

    if (theme === "system") {
      if (systemTheme === "dark") {
        document.documentElement.classList.add("dark");
      } else {
        document.documentElement.classList.add("light");
      }
    }

    if (defaultTheme.includes(theme)) {
      document.documentElement.classList.add(theme);
      return;
    }

    if (darkClasses.includes(theme)) {
      document.documentElement.classList.add("dark");
    }

    document.documentElement.dataset.theme = theme;
  }, [theme, systemTheme]);
};
