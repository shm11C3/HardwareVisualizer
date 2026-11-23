import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { ActionFailure } from "@/components/ActionFailure";

afterEach(cleanup);

describe("ActionFailure", () => {
  it("renders the translated message as an alert under the control", () => {
    render(<ActionFailure message="Could not save this setting." />);

    const alert = screen.getByRole("alert");
    expect(alert).toHaveTextContent("Could not save this setting.");
    expect(alert).toHaveAttribute("data-testid", "action-failure");
  });

  it("offers no retry button: the user uses the control again", () => {
    render(<ActionFailure message="Could not save this setting." />);

    expect(screen.queryByRole("button")).toBeNull();
  });
});
