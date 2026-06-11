import { atom, useAtom } from "jotai";
import { useEffect } from "react";
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

  const toggleMenu = () => {
    setMenuOpen(!isOpen);
  };

  const handleMenuClick = (type: SelectedDisplayType) => {
    setDisplayTarget(type);
    setDisplayTargetAtom(type);
  };

  return {
    isOpen,
    toggleMenu,
    handleMenuClick,
    displayTarget,
  };
};
