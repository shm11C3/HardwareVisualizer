type Equal<T> = (a: T, b: T) => boolean;

/**
 * Whether two arrays hold the same items in the same order.
 *
 * Items are compared with `Object.is` unless `itemEqual` says otherwise, so an
 * array of records can be compared one level deeper with `shallowEqualRecord`.
 * It exists so a producer that rebuilds a value on every tick can hand the
 * previous reference back when nothing changed; Jotai (and React) skip
 * subscribers whose value is `Object.is`-equal.
 */
export const shallowEqualArray = <T>(
  a: readonly T[],
  b: readonly T[],
  itemEqual: Equal<T> = Object.is,
): boolean => {
  if (Object.is(a, b)) {
    return true;
  }
  if (a.length !== b.length) {
    return false;
  }
  return a.every((item, index) => itemEqual(item, b[index] as T));
};

/**
 * Whether two records have the same keys, in the same order, with equal values.
 *
 * Key order counts: consumers read these records through `Object.values` and
 * `Object.entries` (an adapter list in payload order, for one), so a record
 * that merely reordered is a different value to them. Values are compared with
 * `Object.is` unless `valueEqual` says otherwise.
 */
export const shallowEqualRecord = <T extends object>(
  a: T,
  b: T,
  valueEqual: Equal<T[keyof T]> = Object.is,
): boolean => {
  if (Object.is(a, b)) {
    return true;
  }
  const aEntries = Object.entries(a);
  const bEntries = Object.entries(b);
  if (aEntries.length !== bEntries.length) {
    return false;
  }
  return aEntries.every(
    ([key, value], index) =>
      key === bEntries[index]?.[0] && valueEqual(value, bEntries[index][1]),
  );
};
