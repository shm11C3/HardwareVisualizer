import { useSettingsAtom } from "@/atom/useSettingsAtom";
import { Button } from "@/components/ui/button";
import { useBackgroundImage, useBackgroundImageList } from "@/hooks/useBgImage";
import { X } from "@phosphor-icons/react";
import { twMerge } from "tailwind-merge";
import { tv } from "tailwind-variants";

export const BackgroundImageList = () => {
  const { settings, updateSettingAtom } = useSettingsAtom();
  const { backgroundImageList } = useBackgroundImageList();
  const { deleteBackgroundImage } = useBackgroundImage();

  const selectImageVariants = tv({
    base: "relative w-20 h-20 mx-2 rounded-2xl",
    variants: {
      selected: {
        true: "border-2 border-white",
        false: "border border-gray-500",
      },
    },
  });

  return (
    <div className="flex py-3">
      {backgroundImageList.length > 0 && (
        <Button
          className={twMerge(
            selectImageVariants({
              selected: !settings.selectedBackgroundImg,
            }),
            "flex items-center justify-center bg-zinc-300 dark:bg-gray-800 hover:bg-zinc-200 dark:hover:bg-gray-900",
          )}
          onClick={() => {
            updateSettingAtom("selectedBackgroundImg", null);
          }}
        >
          {}
        </Button>
      )}

      {backgroundImageList
        .slice()
        .reverse()
        .map((image) => (
          <div key={image.fileId} className="relative">
            <button
              className="absolute top-0 right-0 p-1 text-white bg-gray-500 bg-opacity-50 rounded-full z-20"
              type="button"
              onClick={() => deleteBackgroundImage(image.fileId)}
            >
              <X />
            </button>
            <button
              type="button"
              className={twMerge(
                selectImageVariants({
                  selected: settings.selectedBackgroundImg === image.fileId,
                }),
                "overflow-hidden",
              )}
              onClick={() =>
                updateSettingAtom("selectedBackgroundImg", image.fileId)
              }
            >
              <img
                src={image.imageData}
                alt={`background image: ${image.fileId}`}
                className="object-cover w-full h-full"
              />
            </button>
          </div>
        ))}
    </div>
  );
};
