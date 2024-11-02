import { useSettingsAtom } from "@/atom/useSettingsAtom";
import { convertFileToBase64 } from "@/lib/file";
import {
  fetchBackgroundImages,
  getBgImage,
  saveBgImage,
} from "@/services/fileSystemService";
import type { BackgroundImage } from "@/types/settingsType";
import { atom, useAtom } from "jotai";
import { useEffect } from "react";

const backgroundImageAtom = atom<string | null>(null);
const uploadedBackgroundImagesAtom = atom<Array<BackgroundImage>>([]);

export const useBackgroundImage = () => {
  const [backgroundImage, setBackgroundImage] = useAtom(backgroundImageAtom);
  const { initBackgroundImages } = useBackgroundImageList();

  const { settings, updateSettingAtom } = useSettingsAtom();

  useEffect(() => {
    const fetchBackgroundImage = async (fileId: string) => {
      try {
        const base64Image = await getBgImage(fileId);
        setBackgroundImage(`data:image/png;base64,${base64Image}`);
      } catch (error) {
        console.error("Failed to load background image:", error);
      }
    };

    if (settings.selectedBackgroundImg) {
      fetchBackgroundImage(settings.selectedBackgroundImg);
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

    const fileId = await saveBgImage(base64Image);
    updateSettingAtom("selectedBackgroundImg", fileId);
    initBackgroundImages();
  };

  return { backgroundImage, saveBackgroundImage };
};

export const useBackgroundImageList = () => {
  const [backgroundImageList, setBackgroundImageList] = useAtom(
    uploadedBackgroundImagesAtom,
  );

  const initBackgroundImages = async () => {
    try {
      const uploadedBackgroundImages = await fetchBackgroundImages();

      const backgroundImagesWithUrl = uploadedBackgroundImages.map((image) => ({
        fileId: image.fileId,
        imageData: `data:image/png;base64,${image.imageData}`,
      }));

      setBackgroundImageList(backgroundImagesWithUrl);
    } catch (error) {
      console.error("Failed to load background images:", error);
    }
  };

  // biome-ignore lint/correctness/useExhaustiveDependencies: <explanation>
  useEffect(() => {
    initBackgroundImages();
  }, []);

  return { backgroundImageList, initBackgroundImages };
};
