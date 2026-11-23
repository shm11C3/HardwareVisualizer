import { commands } from "@/rspc/bindings";
import { isError } from "@/types/result";

export const setCloseToTrayPreference = async (
  value: boolean,
): Promise<boolean> => {
  try {
    const result = await commands.setCloseToTrayPreference(value);
    return !isError(result);
  } catch (error) {
    console.error("Failed to save close-to-tray preference:", error);
    return false;
  }
};
