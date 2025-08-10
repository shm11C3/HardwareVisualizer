import { useEffect } from "react";
import { darkClasses } from "@/consts/style";
import type { Theme } from "@/rspc/bindings";

const defaultTheme = ["dark", "light"];

export const useColorTheme = (theme: Theme) => {
  useEffect(() => {
    document.documentElement.classList.remove(...defaultTheme);
    document.documentElement.dataset.theme = "";

    if (defaultTheme.includes(theme)) {
      document.documentElement.classList.add(theme);
      return;
    }

    if (darkClasses.includes(theme)) {
      document.documentElement.classList.add("dark");
    }

    document.documentElement.dataset.theme = theme;
  }, [theme]);
};
