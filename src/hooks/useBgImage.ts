import { convertFileToBase64 } from "@/lib/file";
import { getBgImage, saveBgImage } from "@/services/fileSystemService";
import { useEffect, useState } from "react";

export const useBackgroundImage = (/** fileId: string */) => {
  const [backgroundImage, setBackgroundImage] = useState<string | null>(null);
  const [uploadedBackgroundImages, setUploadedBackgroundImages] = useState<
    string[]
  >([]);

  useEffect(() => {
    const fetchBackgroundImage = async () => {
      try {
        const base64Image = await getBgImage();
        setBackgroundImage(`data:image/png;base64,${base64Image}`);
      } catch (error) {
        console.error("Failed to load background image:", error);
      }
    };

    fetchBackgroundImage();
  }, []);

  /**
   * 背景画像を保存する
   *
   * @param filePath
   */
  const saveBackgroundImage = async (imageFile: File) => {
    const base64Image = await convertFileToBase64(imageFile);

    const fileId = await saveBgImage(base64Image);
    console.log(fileId);
  };

  return { backgroundImage, saveBackgroundImage };
};
