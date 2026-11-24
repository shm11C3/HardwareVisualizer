/**
 * Convert xxx,yyy,zzz to HEX
 *
 * @param rgb
 * @returns
 *
 * @todo Support rgb(xxx,yyy,zzz) and rgba(xxx,yyy,zzz,a.a) formats
 */
export const RGB2HEX = (rgb: string): string => {
  return `#${rgb
    .split(",")
    .map((value) => Number(value).toString(16).padStart(2, "0"))
    .join("")}`;
};
