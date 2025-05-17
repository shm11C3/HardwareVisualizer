import { getStoreInstance } from "@/lib/tauriStore";
import { useCallback, useEffect, useRef, useState } from "react";

type TauriStore<T> =
  | [value: null, setValue: (newValue: T) => Promise<void>, isPending: true]
  | [value: T, setValue: (newValue: T) => Promise<void>, isPending: false];

export const useTauriStore = <T>(
  key: string,
  defaultValue: T,
): TauriStore<T> => {
  const [value, setValueState] = useState<T | null>(null);
  const [isPending, setIsPending] = useState(true);
  const isMountedRef = useRef(true);

  useEffect(() => {
    isMountedRef.current = true;

    const fetchValue = async () => {
      const store = await getStoreInstance();

      const storedValue = (await store.has(key))
        ? await store.get<T>(key)
        : null;

      if (!storedValue) {
        await store.set(key, defaultValue);
        await store.save();
      }

      if (isMountedRef.current) {
        setValueState(storedValue ?? defaultValue);
        setIsPending(false);
      }
    };

    fetchValue();

    return () => {
      isMountedRef.current = false;
    };
  }, [key, defaultValue]);

  const setValue = useCallback(
    async (newValue: T) => {
      console.log("setValue", newValue);
      const store = await getStoreInstance();
      await store.set(key, newValue);
      await store.save();
      setValueState(newValue);
    },
    [key],
  );

  return isPending ? [null, setValue, true] : [value as T, setValue, false];
};
