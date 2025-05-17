import { useTauriStore } from "@/hooks/useTauriStore";
import type { SelectedDisplayType } from "@/types/ui";
import { atom, useAtom } from "jotai";
import { useCallback, useEffect } from "react";

export const displayTargetAtom = atom<SelectedDisplayType | null>(null);

export const useMenu = () => {
  const [, setDisplayTargetAtom] = useAtom(displayTargetAtom);
  const [isOpen, setMenuOpen] = useTauriStore("sideMenuOpen", false);
  const [displayTarget, setDisplayTarget] = useTauriStore<SelectedDisplayType>(
    "display",
    "dashboard",
  );

  useEffect(() => {
    if (displayTarget) {
      setDisplayTargetAtom(displayTarget);
    }
  }, [displayTarget, setDisplayTargetAtom]);

  const toggleMenu = useCallback(() => {
    setMenuOpen((prev) => !prev);
  }, []);

  const handleMenuClick = useCallback(
    (type: SelectedDisplayType) => {
      setDisplayTarget(type);
      setDisplayTargetAtom(type);
    },
    [setDisplayTarget, setDisplayTargetAtom],
  );

  return {
    isOpen,
    toggleMenu,
    handleMenuClick,
    displayTarget,
  };
};
