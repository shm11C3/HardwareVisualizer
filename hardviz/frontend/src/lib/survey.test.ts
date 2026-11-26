import { describe, expect, it } from "vitest";
import { surveyUrl } from "./survey";

describe("surveyUrl", () => {
  it("opens the English page with the app context", () => {
    expect(
      surveyUrl({ language: "en", appVersion: "1.12.1", os: "windows" }),
    ).toBe(
      "https://hardviz.com/survey/?source=app&app_version=1.12.1&os=windows",
    );
  });

  it("opens the Japanese page for Japanese", () => {
    expect(
      surveyUrl({ language: "ja-JP", appVersion: "1.12.1", os: "macos" }),
    ).toBe(
      "https://hardviz.com/ja/survey/?source=app&app_version=1.12.1&os=macos",
    );
  });

  it("opens the English page for languages the site does not have", () => {
    expect(
      surveyUrl({ language: "ru", appVersion: "1.12.1", os: "linux" }),
    ).toBe(
      "https://hardviz.com/survey/?source=app&app_version=1.12.1&os=linux",
    );
  });
});
