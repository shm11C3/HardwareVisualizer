import { useCallback, useEffect, useRef, useState } from "react";
import { getStoreInstance } from "@/lib/tauriStore";

const KEY_CLOSE_TO_TRAY = "closeToTray";
const KEY_CLOSE_TO_TRAY_CHOICE_MADE = "closeToTrayChoiceMade";

export const setCloseToTrayPreference = async (value: boolean) => {
  const store = await getStoreInstance();
  await store.set(KEY_CLOSE_TO_TRAY, value);
  await store.set(KEY_CLOSE_TO_TRAY_CHOICE_MADE, true);
  await store.save();
};

export const useCloseToTrayPreference = () => {
  const [closeToTray, setCloseToTrayState] = useState(false);
  const [choiceMade, setChoiceMade] = useState(false);
  const [isPending, setIsPending] = useState(true);
  const isMountedRef = useRef(true);

  useEffect(() => {
    isMountedRef.current = true;

    const loadPreference = async () => {
      try {
        const store = await getStoreInstance();
        const storedCloseToTray = (await store.has(KEY_CLOSE_TO_TRAY))
          ? await store.get<boolean>(KEY_CLOSE_TO_TRAY)
          : undefined;
        const storedChoiceMade = (await store.has(
          KEY_CLOSE_TO_TRAY_CHOICE_MADE,
        ))
          ? await store.get<boolean>(KEY_CLOSE_TO_TRAY_CHOICE_MADE)
          : undefined;

        if (isMountedRef.current) {
          setCloseToTrayState(storedCloseToTray ?? false);
          setChoiceMade(storedChoiceMade ?? false);
        }
      } catch (error) {
        console.error("Failed to load close-to-tray preference:", error);
      } finally {
        if (isMountedRef.current) {
          setIsPending(false);
        }
      }
    };

    loadPreference();

    return () => {
      isMountedRef.current = false;
    };
  }, []);

  const setCloseToTray = useCallback(
    async (value: boolean): Promise<boolean> => {
      try {
        await setCloseToTrayPreference(value);
        setCloseToTrayState(value);
        setChoiceMade(true);
        return true;
      } catch (error) {
        console.error("Failed to save close-to-tray preference:", error);
        return false;
      }
    },
    [],
  );

  return {
    closeToTray,
    choiceMade,
    isPending,
    setCloseToTray,
  };
};
