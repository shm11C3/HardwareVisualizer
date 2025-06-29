import { XIcon } from "@phosphor-icons/react";
import { twMerge } from "tailwind-merge";
import { tv } from "tailwind-variants";
import { Button } from "@/components/ui/button";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import { useBackgroundImage, useBackgroundImageList } from "@/hooks/useBgImage";

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
    <>
      {backgroundImageList.length > 0 && (
        <div className="flex max-w-full overflow-x-auto py-3">
          <Button
            className={twMerge(
              selectImageVariants({
                selected: !settings.selectedBackgroundImg,
              }),
              "flex min-w-20 items-center justify-center bg-zinc-300 hover:bg-zinc-200 dark:bg-gray-800 dark:hover:bg-gray-900",
            )}
            onClick={() => {
              updateSettingAtom("selectedBackgroundImg", null);
            }}
          >
            {}
          </Button>

          {backgroundImageList.map((image) => (
            <div key={image.fileId} className="relative">
              <button
                className="absolute top-[-6px] right-[-4px] z-20 cursor-pointer rounded-full bg-gray-500 bg-opacity-80 p-1 text-white"
                type="button"
                onClick={() => deleteBackgroundImage(image.fileId)}
              >
                <XIcon />
              </button>
              <button
                type="button"
                className={twMerge(
                  selectImageVariants({
                    selected: settings.selectedBackgroundImg === image.fileId,
                  }),
                  "overflow-hidden",
                  "cursor-pointer",
                )}
                onClick={() =>
                  updateSettingAtom("selectedBackgroundImg", image.fileId)
                }
              >
                <img
                  src={image.imageData}
                  alt={`background image: ${image.fileId}`}
                  className="h-full w-full object-cover opacity-50"
                />
              </button>
            </div>
          ))}
        </div>
      )}
    </>
  );
};
