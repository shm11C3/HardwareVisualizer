import { getBgImage } from "@/services/fileSystemService";
import { useEffect, useState } from "react";

export const useBackgroundImage = (/** filePath: string */) => {
  const [backgroundImage, setBackgroundImage] = useState<string | null>(null);

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

  return backgroundImage;
};
