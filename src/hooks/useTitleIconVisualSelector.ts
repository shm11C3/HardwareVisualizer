import { atom, useAtomValue, useSetAtom } from "jotai";
import type { SelectedDisplayType } from "@/types/ui";

const showTitleIconAtom = atom<SelectedDisplayType[]>([
  "dashboard",
  "cpuDetail",
  "insights",
  "settings",
]);

export const useTitleIconVisualSelector = () => {
  const setShowTitleIcon = useSetAtom(showTitleIconAtom);
  const visibleTypes = useAtomValue(showTitleIconAtom);

  const isTitleIconVisible = (type: SelectedDisplayType): boolean => {
    return visibleTypes.includes(type);
  };

  const toggleTitleIconVisibility = (
    type: SelectedDisplayType,
    visible: boolean,
  ) => {
    setShowTitleIcon((prev) => {
      if (visible) {
        if (!prev.includes(type)) {
          return [...prev, type];
        }
        return prev;
      }
      return prev.filter((t) => t !== type);
    });
  };

  return { visibleTypes, isTitleIconVisible, toggleTitleIconVisibility };
};
