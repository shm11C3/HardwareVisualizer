/** One power-draw reading per domain, in watts; `null` where the platform reports nothing. */
export type PowerDraw = {
  cpuWatts: number | null;
  gpuWatts: number | null;
  aneWatts: number | null;
  packageWatts: number | null;
};

/** The same domains as padded series, oldest first. */
export type PowerDrawHistory = {
  [K in keyof PowerDraw]: (number | null)[];
};
