import {
  ask as showAsk,
  confirm as showConfirm,
  message as showMessage,
} from "@tauri-apps/plugin-dialog";
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

  const ask = async ({
    title,
    message,
    kind,
  }: {
    title?: TitleType;
    message: string;
    kind?: Kind;
  }): Promise<boolean> => {
    return await showAsk(message, {
      title: title ? t(`error.title.${title}`) : undefined,
      kind,
    });
  };

  const confirm = async ({
    title,
    message,
    kind,
  }: {
    title?: TitleType;
    message: string;
    kind?: Kind;
  }): Promise<boolean> => {
    return await showConfirm(message, {
      title: title ? t(`error.title.${title}`) : undefined,
      kind,
    });
  };

  const message = async ({
    title,
    message,
    kind,
  }: {
    title?: TitleType;
    message: string;
    kind?: Kind;
  }): Promise<void> => {
    return await showMessage(message, {
      title: title ? t(`error.title.${title}`) : undefined,
      kind,
    });
  };

  const error = async (errorMessage: string) => {
    return await message({
      title: "error",
      message: errorMessage,
      kind: "error",
    });
  };

  return { ask, confirm, message, error };
};
