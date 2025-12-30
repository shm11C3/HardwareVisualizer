import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

vi.mock("@/components/ui/progress", () => ({
  Progress: ({ value }: { value?: number | null }) => (
    <div data-testid="progress" data-value={String(value ?? "")} />
  ),
}));

import { UpdateTopBar } from "@/features/updater/UpdateBar";

describe("UpdateTopBar", () => {
  afterEach(() => {
    cleanup();
  });

  it("percent を 0..100 にクランプし、表示%と Progress value に反映する", () => {
    const { rerender } = render(<UpdateTopBar percent={-10} />);
    expect(screen.getByText("0%")).toBeInTheDocument();
    expect(screen.getByTestId("progress").getAttribute("data-value")).toBe("0");

    rerender(<UpdateTopBar percent={200} />);
    expect(screen.getByText("100%")).toBeInTheDocument();
    expect(screen.getByTestId("progress").getAttribute("data-value")).toBe(
      "100",
    );
  });

  it("表示%は Math.round 後の値になる", () => {
    const { rerender } = render(<UpdateTopBar percent={33.4} />);
    expect(screen.getByText("33%")).toBeInTheDocument();

    rerender(<UpdateTopBar percent={33.6} />);
    expect(screen.getByText("34%")).toBeInTheDocument();
  });

  it("transferredBytes と totalBytes が揃っている時だけサイズ表記を出す", () => {
    const { rerender } = render(
      <UpdateTopBar percent={10} transferredBytes={1024n} totalBytes={2048n} />,
    );
    expect(screen.getByText("1.0 KB / 2.0 KB")).toBeInTheDocument();

    rerender(
      <UpdateTopBar percent={10} transferredBytes={1024n} totalBytes={null} />,
    );
    expect(screen.queryByText("1.0 KB / 2.0 KB")).not.toBeInTheDocument();

    rerender(<UpdateTopBar percent={10} />);
    expect(screen.queryByText("1.0 KB / 2.0 KB")).not.toBeInTheDocument();
  });

  it("基本文言を表示する", () => {
    render(<UpdateTopBar percent={0} />);
    expect(screen.getByText("Downloading update…")).toBeInTheDocument();
  });
});
