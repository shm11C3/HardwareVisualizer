import { archivePeriods } from "@/features/hardware/consts/chart";

export type ArchivePeriod = (typeof archivePeriods)[number];

const archivePeriodValues = new Set<number>(archivePeriods);

export const normalizeArchivePeriod = (
  value: ArchivePeriod | number | string | null | undefined,
): ArchivePeriod | null => {
  if (value == null) {
    return null;
  }

  const numericValue = typeof value === "number" ? value : Number(value);
  if (!Number.isFinite(numericValue)) {
    return null;
  }

  return archivePeriodValues.has(numericValue)
    ? (numericValue as ArchivePeriod)
    : null;
};

export const coercePeriodMinutes = (value: number | string): number => {
  const numericValue = typeof value === "number" ? value : Number(value);
  if (
    !Number.isInteger(numericValue) ||
    numericValue < 0 ||
    numericValue > 4_294_967_295
  ) {
    throw new Error(`Invalid period minutes: ${String(value)}`);
  }

  return numericValue;
};
