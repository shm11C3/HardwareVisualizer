/**
 * xxx,yyy,zzz を HEX に変換する
 *
 * @param rgb
 * @returns
 *
 * @todo rbg(xxx,yyy,zzz) 形式や rgba(xxx,yyy,zzz,a.a) 形式に対応する
 */
export const RGB2HEX = (rgb: string): string => {
  return `#${rgb
    .split(",")
    .map((value) => Number(value).toString(16).padStart(2, "0"))
    .join("")}`;
};
