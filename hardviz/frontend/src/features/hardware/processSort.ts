export type ProcessSortConfig<T> = {
  key: keyof T;
  direction: "ascending" | "descending";
};

/**
 * Returns a sorted copy. A null value (an unavailable reading, e.g. GPU usage)
 * sorts last in both directions so it never reads as the smallest or largest.
 * Shared by the Live Process Table and Process Insight.
 */
export const sortProcesses = <T>(
  processes: readonly T[],
  sortConfig: ProcessSortConfig<T> | null,
): T[] => {
  const sorted = [...processes];
  if (sortConfig === null) {
    return sorted;
  }

  const sign = sortConfig.direction === "ascending" ? 1 : -1;
  sorted.sort((a, b) => {
    const aValue = a[sortConfig.key];
    const bValue = b[sortConfig.key];

    if (aValue === null || bValue === null) {
      if (aValue === bValue) {
        return 0;
      }
      return aValue === null ? 1 : -1;
    }

    // Compare directly for numbers
    if (typeof aValue === "number" && typeof bValue === "number") {
      return sign * (aValue - bValue);
    }

    // Convert to number if stored as string
    if (typeof aValue === "string" && typeof bValue === "string") {
      const aNumber = Number.parseFloat(aValue);
      const bNumber = Number.parseFloat(bValue);

      // Compare as numbers if not NaN to avoid string comparison of decimals
      if (!Number.isNaN(aNumber) && !Number.isNaN(bNumber)) {
        return sign * (aNumber - bNumber);
      }

      // Compare as strings if not numbers
      return sign * aValue.localeCompare(bValue);
    }

    // Don't change order if types differ
    return 0;
  });
  return sorted;
};
