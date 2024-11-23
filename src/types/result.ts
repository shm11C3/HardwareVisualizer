import type { Result } from "@/rspc/bindings";

export const isResult = <T, E>(unknown: unknown): unknown is Result<T, E> => {
  const result = unknown as Result<T, E>;

  return (
    (result.status === "ok" && result.data !== undefined) ||
    (result.status === "error" && result.error !== undefined)
  );
};

export const isOk = <T, E>(
  result: Result<T, E>,
): result is { status: "ok"; data: T } => {
  return result.status === "ok";
};

export const isError = <T, E>(
  result: Result<T, E>,
): result is { status: "error"; error: E } => {
  return result.status === "error";
};
