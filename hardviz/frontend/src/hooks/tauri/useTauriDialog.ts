import {
  type MessageDialogResult,
  ask as showAsk,
  confirm as showConfirm,
  message as showMessage,
} from "@tauri-apps/plugin-dialog";
import { useCallback } from "react";
import { useTranslation } from "react-i18next";

type Kind = "info" | "warning" | "error";
type TitleType =
  | "info"
  | "confirm"
  | "success"
  | "warning"
  | "error"
  | "unexpected";

export const useTauriDialog = () => {
  const { t } = useTranslation();

  const ask = useCallback(
    async ({
      title,
      message,
      kind,
    }: {
      title?: TitleType;
      message: string;
      kind?: Kind;
    }): Promise<boolean> => {
      return await showAsk(message, {
        ...(title !== undefined && { title: t(`error.title.${title}`) }),
        ...(kind !== undefined && { kind }),
      });
    },
    [t],
  );

  const confirm = useCallback(
    async ({
      title,
      message,
      kind,
    }: {
      title?: TitleType;
      message: string;
      kind?: Kind;
    }): Promise<boolean> => {
      return await showConfirm(message, {
        ...(title !== undefined && { title: t(`error.title.${title}`) }),
        ...(kind !== undefined && { kind }),
      });
    },
    [t],
  );

  const message = useCallback(
    async ({
      title,
      message,
      kind,
    }: {
      title?: TitleType;
      message: string;
      kind?: Kind;
    }): Promise<MessageDialogResult> => {
      return await showMessage(message, {
        ...(title !== undefined && { title: t(`error.title.${title}`) }),
        ...(kind !== undefined && { kind }),
      });
    },
    [t],
  );

  const error = useCallback(
    async (errorMessage: string) => {
      return await message({
        title: "error",
        message: errorMessage,
        kind: "error",
      });
    },
    [message],
  );

  return { ask, confirm, message, error };
};
