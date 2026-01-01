import { listen } from "@tauri-apps/api/event";
import { message } from "@tauri-apps/plugin-dialog";
import { useEffect } from "react";

/**
 * Listen for error events from the backend and display error dialogs
 */
export const useErrorModalListener = () => {
  useEffect(() => {
    const unListen = listen("error_event", (event) => {
      const { title, message: errorMessage } = event.payload as {
        title: string;
        message: string;
      };

      message(errorMessage, {
        title: title,
        kind: "error",
      });
    });

    return () => {
      unListen.then((off) => off());
    };
  }, []);
};
