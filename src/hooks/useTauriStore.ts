import { useCallback, useEffect, useState } from "react";
import { getStoreInstance } from "@/lib/tauriStore";

/**
 * `loadFailed` is true when the initial read threw and `value` is therefore
 * the default rather than what is on disk. A settled default is otherwise
 * indistinguishable from a successful load of an absent key, so consumers
 * that write state derived from `value` back to the store must skip that
 * write-back while `loadFailed` is set: persisting there would overwrite
 * the user's stored intent with a fallback (DP-06).
 */
type TauriStore<T> =
  | [
      value: null,
      setValue: (newValue: T) => Promise<void>,
      isPending: true,
      loadFailed: false,
    ]
  | [
      value: T,
      setValue: (newValue: T) => Promise<void>,
      isPending: false,
      loadFailed: boolean,
    ];

export const useTauriStore = <T>(
  key: string,
  defaultValue: T,
): TauriStore<T> => {
  const [value, setValueState] = useState<T | null>(null);
  const [isPending, setIsPending] = useState(true);
  const [loadFailed, setLoadFailed] = useState(false);

  useEffect(() => {
    // Effect-local rather than a shared ref: a ref is reset to "live" by the
    // next effect run when `key` changes, which let an earlier read that
    // finished last overwrite the newer key's value.
    let isCancelled = false;

    const fetchValue = async () => {
      let resolvedValue = defaultValue;
      let failed = false;

      try {
        const store = await getStoreInstance();

        const storedValue = (await store.has(key))
          ? await store.get<T>(key)
          : null;

        if (!storedValue) {
          await store.set(key, defaultValue);
          await store.save();
        }

        resolvedValue = storedValue ?? defaultValue;
      } catch (error) {
        // A failed store read must still settle the hook. Consumers gate on
        // isPending, so staying pending would hide their UI for the session.
        failed = true;
        console.error(`Failed to read Tauri Store key "${key}":`, error);
      }

      if (isCancelled) return;
      setValueState(resolvedValue);
      setLoadFailed(failed);
      setIsPending(false);
    };

    fetchValue();

    return () => {
      isCancelled = true;
    };
  }, [key, defaultValue]);

  const setValue = useCallback(
    async (newValue: T) => {
      const store = await getStoreInstance();
      await store.set(key, newValue);
      await store.save();
      setValueState(newValue);
    },
    [key],
  );

  return isPending
    ? [null, setValue, true, false]
    : [value as T, setValue, false, loadFailed];
};
