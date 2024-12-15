import { load } from "@tauri-apps/plugin-store";
import { useEffect, useState } from "react";

const storePromise = load("store.json", { autoSave: true });

export const useTauriStore = <T>(
  key: string,
  defaultValue: T,
): [T | null, (newValue: T) => Promise<void>] => {
  const [stateValue, setStateValue] = useState<T | null>(null);

  useEffect(() => {
    const fetchToggleState = async () => {
      const store = await storePromise;

      if (await store.has(key)) {
        const storedValue = await store.get<T>(key);
        setStateValue(storedValue ?? defaultValue);
      } else {
        await store.set(key, defaultValue);
        await store.save();

        setStateValue(defaultValue);
      }
    };

    fetchToggleState();
  }, [key, defaultValue]);

  const setValue = async (newValue: T) => {
    const store = await storePromise;
    await store.set(key, newValue);
    await store.save();

    setStateValue(newValue);
  };

  return [stateValue, setValue];
};
