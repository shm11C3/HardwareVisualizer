import { atom, useAtom } from "jotai";
import { useEffect, useRef } from "react";
import { getStoreInstance } from "@/lib/tauriStore";

export const DATABASE_CONVERSION_NOTICE_SHOWN_STORE_KEY =
  "databaseConversionCompleteNoticeShown";

/** `null` means not loaded from the Tauri Store yet. */
const noticeShownAtom = atom<boolean | null>(null);

/**
 * Whether the #2136 conversion-complete notice has already been shown,
 * backed by `DATABASE_CONVERSION_NOTICE_SHOWN_STORE_KEY` in the Tauri
 * Store (UI-local state, not an Application Preference).
 *
 * The app-root prompt dialog (`DatabaseConversionPromptDialog`) and the
 * Settings entry point (`DatabaseConversionSettings`) can both be mounted
 * at once - each drives its own `useDatabaseConversion` instance and so
 * has its own `justCompleted`, which is fine, but a `useTauriStore` call
 * per mount would give each its own copy of the persisted "shown" flag
 * too. Two independent copies can both read "not shown yet" and both
 * render the notice before either mount's dismissal is persisted. A
 * single shared Jotai atom, loaded once by whichever mount asks first
 * and read/written by every mount after that, keeps exactly one answer
 * in memory for the whole app.
 */
export const useDatabaseConversionNoticeShown = (): [
  shown: boolean,
  setShown: (value: boolean) => Promise<void>,
  isPending: boolean,
] => {
  const [shown, setShownAtom] = useAtom(noticeShownAtom);
  const loadStarted = useRef(false);

  useEffect(() => {
    if (shown !== null || loadStarted.current) {
      return;
    }
    loadStarted.current = true;

    (async () => {
      const store = await getStoreInstance();
      const stored = (await store.has(
        DATABASE_CONVERSION_NOTICE_SHOWN_STORE_KEY,
      ))
        ? await store.get<boolean>(DATABASE_CONVERSION_NOTICE_SHOWN_STORE_KEY)
        : null;

      if (stored == null) {
        await store.set(DATABASE_CONVERSION_NOTICE_SHOWN_STORE_KEY, false);
        await store.save();
      }

      setShownAtom(stored ?? false);
    })();
  }, [shown, setShownAtom]);

  const setShown = async (value: boolean) => {
    // Updates the shared atom first (synchronously visible to every other
    // mount) and persists after, mirroring `useTauriStore`'s own
    // optimistic-update shape.
    setShownAtom(value);
    const store = await getStoreInstance();
    await store.set(DATABASE_CONVERSION_NOTICE_SHOWN_STORE_KEY, value);
    await store.save();
  };

  return [shown ?? false, setShown, shown === null];
};
