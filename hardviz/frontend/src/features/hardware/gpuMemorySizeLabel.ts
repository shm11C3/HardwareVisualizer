/**
 * Translation key for an inventory GPU's `memorySize`. On Windows the value is
 * DXGI's shared system memory budget, which is the same for every adapter, so
 * the generic label reads as the adapter's own capacity. Linux reports total
 * VRAM and macOS mixes shared and dedicated figures, so they keep the generic
 * label.
 */
export const gpuMemorySizeLabelKey = (os: string) =>
  os === "windows" ? "shared.memorySizeShared" : "shared.memorySize";
