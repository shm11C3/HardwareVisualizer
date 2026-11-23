export type LinkSpeedUnit = "megabitsPerSecond" | "gigabitsPerSecond";

export type FormattedLinkSpeed = {
  /** Locale-formatted number without a unit. */
  value: string;
  unit: LinkSpeedUnit;
};

const BITS_PER_MEGABIT = 1_000_000;
const BITS_PER_GIGABIT = 1_000_000_000;

const formatNumber = (
  value: number,
  locale: string,
  maximumFractionDigits: number,
): string =>
  new Intl.NumberFormat(locale, {
    minimumFractionDigits: 0,
    maximumFractionDigits,
  }).format(value);

/**
 * Splits a link speed in bits per second into a display number and a unit key.
 * Decimal units (link speeds are marketed in powers of ten). Gb/s keeps one
 * fraction digit; Mb/s from 1 up is whole (867 Mb/s), because a Wi-Fi rate's
 * tenths are noise. Below 1 Mb/s one fraction digit keeps the value non-zero.
 */
export const formatLinkSpeed = (
  bitsPerSecond: number,
  locale: string,
): FormattedLinkSpeed => {
  const megabits = bitsPerSecond / BITS_PER_MEGABIT;

  if (bitsPerSecond < BITS_PER_GIGABIT && Math.round(megabits) < 1000) {
    return {
      value: formatNumber(megabits, locale, megabits < 1 ? 1 : 0),
      unit: "megabitsPerSecond",
    };
  }

  return {
    value: formatNumber(bitsPerSecond / BITS_PER_GIGABIT, locale, 1),
    unit: "gigabitsPerSecond",
  };
};
