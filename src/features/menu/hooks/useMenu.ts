import { atom, useAtom } from "jotai";
import { useCallback, useEffect } from "react";
import { useTauriStore } from "@/hooks/useTauriStore";
import type { SelectedDisplayType } from "@/types/ui";

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
    setMenuOpen(!isOpen);
  }, [isOpen, setMenuOpen]);

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
