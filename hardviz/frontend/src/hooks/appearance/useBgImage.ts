import { useAtom } from "jotai";
import { useCallback, useEffect } from "react";
import { useSettingsAtom } from "@/hooks/settings/useSettingsAtom";
import { convertFileToBase64 } from "@/lib/file";
import { commands } from "@/rspc/bindings";
import {
  type BackgroundImageAction,
  backgroundImageAtom,
  backgroundImageFailuresAtom,
  uploadedBackgroundImagesAtom,
} from "@/store/backgroundImage";
import { isError, isOk } from "@/types/result";

/**
 * Records a failed background image action for the control that owns it, and
 * clears it when the same kind succeeds later. The detail only goes to the log.
 */
const useBackgroundImageFailure = () => {
  const [failures, setFailures] = useAtom(backgroundImageFailuresAtom);

  const recordFailure = (action: BackgroundImageAction, reason: unknown) => {
    console.error(`Background image ${action} failed:`, reason);
    setFailures((current) => new Set(current).add(action));
  };
  const clearFailure = (action: BackgroundImageAction) => {
    setFailures((current) => {
      if (!current.has(action)) return current;
      const next = new Set(current);
      next.delete(action);
      return next;
    });
  };

  return { failures, recordFailure, clearFailure };
};

export const useBackgroundImage = () => {
  const { failures, recordFailure, clearFailure } = useBackgroundImageFailure();
  const [backgroundImage, setBackgroundImage] = useAtom(backgroundImageAtom);
  const { initBackgroundImages, backgroundImageList, setBackgroundImageList } =
    useBackgroundImageList();

  const { settings, updateSettingAtom } = useSettingsAtom();

  const initBackgroundImage = useCallback(async () => {
    if (settings.selectedBackgroundImg) {
      const base64Image = await commands.getBackgroundImage(
        settings.selectedBackgroundImg,
      );

      if (isOk(base64Image)) {
        setBackgroundImage(`data:image/png;base64,${base64Image.data}`);
      } else {
        // An automatic read with no control to report under, and a missing
        // background is cosmetic: log only.
        console.error("Failed to load background image:", base64Image.error);
      }
    } else {
      setBackgroundImage(null);
    }
  }, [setBackgroundImage, settings.selectedBackgroundImg]);

  /**
   * Save background image
   *
   * @param imageFile
   * @returns whether the image was saved, so the upload form knows whether
   * to keep the chosen file for another try
   */
  const saveBackgroundImage = async (imageFile: File) => {
    try {
      const base64Image = await convertFileToBase64(imageFile);

      const fileId = await commands.saveBackgroundImage(base64Image);

      if (isError(fileId)) {
        recordFailure("upload", fileId.error);
        return false;
      }

      clearFailure("upload");
      updateSettingAtom("selectedBackgroundImg", fileId.data);
      initBackgroundImages();
      return true;
    } catch (err) {
      recordFailure("upload", err);
      return false;
    }
  };

  const deleteBackgroundImage = async (fileId: string) => {
    try {
      const result = await commands.deleteBackgroundImage(fileId);
      if (isError(result)) {
        recordFailure("delete", result.error);
        return;
      }
      clearFailure("delete");
      const newBackgroundImages = backgroundImageList.filter(
        (v) => v.fileId !== fileId,
      );
      setBackgroundImageList(newBackgroundImages);
    } catch (err) {
      recordFailure("delete", err);
      return;
    }

    // Deselect only once the file is really gone: a failed delete leaves the
    // image in the list and keeps it selected.
    if (fileId === settings.selectedBackgroundImg) {
      updateSettingAtom("selectedBackgroundImg", null);
    }
  };

  return {
    failures,
    backgroundImage,
    saveBackgroundImage,
    initBackgroundImage,
    deleteBackgroundImage,
  };
};

export const useBackgroundImageList = () => {
  const [backgroundImageList, setBackgroundImageList] = useAtom(
    uploadedBackgroundImagesAtom,
  );

  const initBackgroundImages = async () => {
    const uploadedBackgroundImages = await commands.getBackgroundImages();

    if (isError(uploadedBackgroundImages)) {
      // Automatic read: log only.
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

  // biome-ignore lint/correctness/useExhaustiveDependencies: This effect runs only once to initialize the background images
  useEffect(() => {
    initBackgroundImages();
  }, []);

  return { backgroundImageList, initBackgroundImages, setBackgroundImageList };
};
