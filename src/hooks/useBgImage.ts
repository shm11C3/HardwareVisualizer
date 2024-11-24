import { useSettingsAtom } from "@/atom/useSettingsAtom";
import { convertFileToBase64 } from "@/lib/file";
import { commands } from "@/rspc/bindings";
import { isError, isOk } from "@/types/result";
import type { BackgroundImage } from "@/types/settingsType";
import { atom, useAtom } from "jotai";
import { useCallback, useEffect } from "react";
import { useTauriDialog } from "./useTauriDialog";

const backgroundImageAtom = atom<string | null>(null);
const uploadedBackgroundImagesAtom = atom<Array<BackgroundImage>>([]);

export const useBackgroundImage = () => {
  const { error } = useTauriDialog();
  const [backgroundImage, setBackgroundImage] = useAtom(backgroundImageAtom);
  const { initBackgroundImages, backgroundImageList, setBackgroundImageList } =
    useBackgroundImageList();

  const { settings, updateSettingAtom } = useSettingsAtom();

  // biome-ignore lint/correctness/useExhaustiveDependencies: <explanation>
  const initBackgroundImage = useCallback(async () => {
    if (settings.selectedBackgroundImg) {
      const base64Image = await commands.getBackgroundImage(
        settings.selectedBackgroundImg,
      );

      if (isOk(base64Image)) {
        setBackgroundImage(`data:image/png;base64,${base64Image.data}`);
      } else {
        error(base64Image.error);
        console.error("Failed to load background image:", base64Image.error);
      }
    } else {
      setBackgroundImage(null);
    }
  }, [setBackgroundImage, settings.selectedBackgroundImg]);

  /**
   * 背景画像を保存する
   *
   * @param filePath
   */
  const saveBackgroundImage = async (imageFile: File) => {
    const base64Image = await convertFileToBase64(imageFile);

    const fileId = await commands.saveBackgroundImage(base64Image);

    if (isError(fileId)) {
      error(fileId.error);
      console.error("Failed to save background image:", fileId.error);
      return;
    }

    updateSettingAtom("selectedBackgroundImg", fileId.data);
    initBackgroundImages();
  };

  const deleteBackgroundImage = async (fileId: string) => {
    // 選択中のファイルを削除する場合は選択を解除
    if (fileId === settings.selectedBackgroundImg) {
      updateSettingAtom("selectedBackgroundImg", null);
    }

    try {
      await commands.deleteBackgroundImage(fileId);
      const newBackgroundImages = backgroundImageList.filter(
        (v) => v.fileId !== fileId,
      );
      setBackgroundImageList(newBackgroundImages);
    } catch (err) {
      error(err as string);
      console.error("Failed to delete background image:", err);
    }
  };

  return {
    backgroundImage,
    saveBackgroundImage,
    initBackgroundImage,
    deleteBackgroundImage,
  };
};

export const useBackgroundImageList = () => {
  const { error } = useTauriDialog();
  const [backgroundImageList, setBackgroundImageList] = useAtom(
    uploadedBackgroundImagesAtom,
  );

  const initBackgroundImages = async () => {
    const uploadedBackgroundImages = await commands.getBackgroundImages();

    if (isError(uploadedBackgroundImages)) {
      error(uploadedBackgroundImages.error);
      console.error(
        "Failed to load background images:",
        uploadedBackgroundImages.error,
      );
      return;
    }

    const backgroundImagesWithUrl = uploadedBackgroundImages.data.map(
      (image) => ({
        fileId: image.fileId,
        imageData: `data:image/png;base64,${image.imageData}`,
      }),
    );

    setBackgroundImageList(backgroundImagesWithUrl);
  };

  // biome-ignore lint/correctness/useExhaustiveDependencies: <explanation>
  useEffect(() => {
    initBackgroundImages();
  }, []);

  return { backgroundImageList, initBackgroundImages, setBackgroundImageList };
};
