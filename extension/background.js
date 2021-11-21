let baseUrl = "http://localhost:8080/r/";
let browser = undefined;
if (browser === undefined && chrome !== undefined) {
  browser = chrome;
}
if (browser === undefined && firefox !== undefined) {
  browser = firefox;
}

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

browser.contextMenus.create({
  title: "Clean Read",
  contexts: ["link"],
  onclick: ((info, _b) => {
    browser.storage.local.get(['server_url'], function (result) {
      browser.tabs.create({
        url: genURL(result.server_url) + btoa(info.linkUrl).split("/").join("_")
      });
    });
  })
});