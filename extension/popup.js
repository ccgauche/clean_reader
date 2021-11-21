// Initialize button with user's preferred color
let changeColor = document.getElementById("svurl");
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

changeColor.addEventListener("keyup", (e) => {
    let v = genURL(changeColor.value);
    changeColor.value = v;
    browser.storage.local.set({
        "server_url": v
    });
})

browser.storage.local.get("server_url", ({
    server_url
}) => {
    changeColor.value = server_url;
});
browser.storage.onChanged.addListener(function (changes, namespace) {
    for (let [key, {
            _,
            newValue
        }] of Object.entries(changes)) {
        if (key === "server_url") {
            changeColor.value = genURL(newValue);
        }
    }
});