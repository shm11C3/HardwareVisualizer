import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { LoadFailure } from "@/components/LoadFailure";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

afterEach(cleanup);

describe("LoadFailure", () => {
  it("renders the translated message where the data would have appeared", () => {
    render(<LoadFailure message="Could not load the data." />);

    expect(screen.getByTestId("load-failure")).toHaveTextContent(
      "Could not load the data.",
    );
  });

  it("calls onRetry when the retry button is pressed", () => {
    const onRetry = vi.fn();
    render(
      <LoadFailure message="Could not load the data." onRetry={onRetry} />,
    );

    fireEvent.click(screen.getByRole("button", { name: "shared.retry" }));

    expect(onRetry).toHaveBeenCalledTimes(1);
  });

  it("offers no retry button when the read retries on its own", () => {
    render(<LoadFailure message="Could not load the data." />);

    expect(screen.queryByRole("button")).toBeNull();
  });
});
