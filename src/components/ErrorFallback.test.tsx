import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import ErrorFallback from "@/components/ErrorFallback";

describe("ErrorFallback", () => {
  it("should render error message when error occurs", () => {
    const testError = new Error("Test error message");

    render(<ErrorFallback error={testError} resetErrorBoundary={() => {}} />);

    expect(screen.getByRole("heading", { level: 2 })).toHaveTextContent(
      "An unexpected error has occurred.",
    );
    expect(screen.getByText("Test error message")).toBeInTheDocument();
  });

  it("should render pre-formatted error message", () => {
    const testError = new Error("Formatted error");

    render(<ErrorFallback error={testError} resetErrorBoundary={() => {}} />);

    const preElement = screen.getByText("Formatted error");
    expect(preElement.tagName.toLowerCase()).toBe("pre");
  });

  it("should handle errors with multiline messages", () => {
    const multilineError = new Error("Line 1\nLine 2\nLine 3");

    render(
      <ErrorFallback error={multilineError} resetErrorBoundary={() => {}} />,
    );

    expect(screen.getByText(/Line 1/)).toBeInTheDocument();
    expect(screen.getByText(/Line 2/)).toBeInTheDocument();
    expect(screen.getByText(/Line 3/)).toBeInTheDocument();
  });
});
