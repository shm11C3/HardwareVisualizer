import { useSetAtom } from "jotai";
import { displayTargetAtom } from "@/store/navigation";

/**
 * Switches the screen shown right now. This writes the shared in-memory
 * selection only; persisting the choice to the Tauri Store is the caller's
 * job (see `useMenu`).
 */
export const useDisplayTargetSetter = () => {
  const setDisplayTargetAtom = useSetAtom(displayTargetAtom);
  return { setDisplayTargetAtom };
};
