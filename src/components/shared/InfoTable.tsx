import { minOpacity } from "@/consts/style";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import { cn } from "@/lib/utils";

export const InfoTable = ({
  data,
  className,
}: { data: { [key: string]: string | number }; className?: string }) => {
  const { settings } = useSettingsAtom();

  return (
    <div
      className={cn("grid grid-cols-2 gap-2 px-4 pt-2 pb-4", className)}
      style={{
        opacity:
          settings.selectedBackgroundImg != null
            ? Math.max(
                (1 - settings.backgroundImgOpacity / 100) ** 2,
                minOpacity,
              )
            : 1,
      }}
    >
      {Object.keys(data).map((key) => (
        <div key={key}>
          <p className="text-slate-900 text-sm dark:text-slate-400">{key}</p>
          <p>{data[key]}</p>
        </div>
      ))}
    </div>
  );
};
