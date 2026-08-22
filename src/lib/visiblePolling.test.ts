import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { startVisiblePolling } from "@/lib/visiblePolling";

const setDocumentHidden = (hidden: boolean) => {
  Object.defineProperty(document, "hidden", {
    configurable: true,
    value: hidden,
  });
};

const goHidden = () => {
  setDocumentHidden(true);
  document.dispatchEvent(new Event("visibilitychange"));
};

const goVisible = () => {
  setDocumentHidden(false);
  document.dispatchEvent(new Event("visibilitychange"));
};

describe("startVisiblePolling", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    setDocumentHidden(false);
  });

  afterEach(() => {
    vi.useRealTimers();
    setDocumentHidden(false);
  });

  it("polls immediately and on the interval while visible", () => {
    const poll = vi.fn();

    const stop = startVisiblePolling(poll, 10_000);

    expect(poll).toHaveBeenCalledTimes(1);

    vi.advanceTimersByTime(20_000);
    expect(poll).toHaveBeenCalledTimes(3);

    stop();
  });

  it("stops polling while the document is hidden", () => {
    const poll = vi.fn();

    const stop = startVisiblePolling(poll, 10_000);
    poll.mockClear();

    goHidden();
    vi.advanceTimersByTime(60_000);

    expect(poll).not.toHaveBeenCalled();

    stop();
  });

  it("refreshes immediately and resumes when the document becomes visible", () => {
    const poll = vi.fn();

    const stop = startVisiblePolling(poll, 10_000);
    goHidden();
    poll.mockClear();

    goVisible();
    expect(poll).toHaveBeenCalledTimes(1);

    vi.advanceTimersByTime(10_000);
    expect(poll).toHaveBeenCalledTimes(2);

    stop();
  });

  it("does not poll when it starts while already hidden", () => {
    const poll = vi.fn();
    setDocumentHidden(true);

    const stop = startVisiblePolling(poll, 10_000);

    expect(poll).not.toHaveBeenCalled();

    vi.advanceTimersByTime(60_000);
    expect(poll).not.toHaveBeenCalled();

    stop();
  });

  it("keeps a single interval when repeated visibility events arrive", () => {
    const poll = vi.fn();

    const stop = startVisiblePolling(poll, 10_000);
    poll.mockClear();

    goVisible();
    goVisible();
    poll.mockClear();

    vi.advanceTimersByTime(10_000);
    expect(poll).toHaveBeenCalledTimes(1);

    stop();
  });

  it("stops polling and detaches its listener after stop", () => {
    const poll = vi.fn();

    const stop = startVisiblePolling(poll, 10_000);
    stop();
    poll.mockClear();

    vi.advanceTimersByTime(60_000);
    goHidden();
    goVisible();
    vi.advanceTimersByTime(60_000);

    expect(poll).not.toHaveBeenCalled();
  });
});
