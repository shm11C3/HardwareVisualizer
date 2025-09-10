import { render, screen, cleanup } from "@testing-library/react";
import { describe, expect, it, afterEach } from "vitest";
import { Button } from "@/components/ui/button";

describe("Button", () => {
  afterEach(() => {
    cleanup();
  });

  it("should render with default props", () => {
    render(<Button>Click me</Button>);

    const button = screen.getByRole("button");
    expect(button).toHaveTextContent("Click me");
    expect(button).toHaveClass("inline-flex", "items-center", "justify-center");
  });

  it("should apply variant classes", () => {
    render(<Button variant="destructive">Delete</Button>);

    const button = screen.getByText("Delete");
    expect(button).toHaveClass("bg-red-500");
  });

  it("should apply size classes", () => {
    render(<Button size="lg">Large Button</Button>);

    const button = screen.getByText("Large Button");
    expect(button).toHaveClass("h-11", "px-8");
  });

  it("should handle disabled state", () => {
    render(<Button disabled>Disabled</Button>);

    const button = screen.getByText("Disabled");
    expect(button).toBeDisabled();
    expect(button).toHaveClass("disabled:opacity-50");
  });

  it("should render as child component when asChild is true", () => {
    render(
      <Button asChild>
        <a href="/test">Link Button</a>
      </Button>,
    );

    const link = screen.getByRole("link");
    expect(link).toHaveTextContent("Link Button");
    expect(link).toHaveAttribute("href", "/test");
  });

  it("should merge custom className with button variants", () => {
    render(<Button className="custom-class">Custom</Button>);

    const button = screen.getByText("Custom");
    expect(button).toHaveClass("custom-class");
    expect(button).toHaveClass("inline-flex"); // default classes should still be present
  });

  it("should handle outline variant", () => {
    render(<Button variant="outline">Outline</Button>);

    const button = screen.getByText("Outline");
    expect(button).toHaveClass("border");
  });

  it("should handle ghost variant", () => {
    render(<Button variant="ghost">Ghost</Button>);

    const button = screen.getByText("Ghost");
    expect(button).toHaveClass("hover:bg-[var(--muted)]");
  });
});
