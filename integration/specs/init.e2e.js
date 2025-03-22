import { expect } from "chai";

describe("Application Initial Render Test", () => {
  it("should render the main page on startup", async () => {
    await browser.url("http://localhost:1520");

    const handles = await browser.getWindowHandles();
    await browser.switchToWindow(handles[0]);

    const toggleButton = await browser.$('[aria-label="Toggle Side Menu"]');
    await toggleButton.waitForExist({ timeout: 10000 });
    expect(await toggleButton.isDisplayed()).to.be.true;

    await toggleButton.click();
    const sideMenu = await $('[data-testid="menu-dashboard"]');
    //expect(await sideMenu.isDisplayed()).to.be.true;
  });
});
