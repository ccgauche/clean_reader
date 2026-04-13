// MV3 service worker. `chrome` is available here; `browser` is not (without a
// polyfill), so we just alias it for consistency with the rest of the codebase.
const browser = chrome;

const MENU_ID = "clean-read-link";

function genURL(x) {
    if (x === undefined || x === "undefined" || x === "http://undefined/r/") {
        x = "http://localhost:8080/r/";
    }
    if (!x.startsWith("http://") && !x.startsWith("https://")) {
        x = "http://" + x;
    }
    if (x.endsWith("/m/")) {
        x = x.substring(0, x.length - 3);
    }
    if (x.endsWith("/m")) {
        x = x.substring(0, x.length - 2);
    }
    if (!x.endsWith("/r/") && !x.endsWith("/r")) {
        x = x + "/r/";
    }
    if (x.endsWith("/r")) {
        x = x + "/";
    }
    return x;
}

// Context menus must be registered at install/update time in MV3; calling
// chrome.contextMenus.create at the top level of a service worker would race
// with worker restarts.
browser.runtime.onInstalled.addListener(() => {
    browser.contextMenus.create({
        id: MENU_ID,
        title: "Clean Read",
        contexts: ["link"]
    });
});

browser.contextMenus.onClicked.addListener((info, _tab) => {
    if (info.menuItemId !== MENU_ID) return;
    browser.storage.local.get(["server_url"], (result) => {
        const target = genURL(result.server_url) + btoa(info.linkUrl).split("/").join("_");
        browser.tabs.create({ url: target });
    });
});
