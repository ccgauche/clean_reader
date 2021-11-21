(function () {
    let browser = undefined;
    if (browser === undefined && chrome !== undefined) {
        browser = chrome;
    }
    if (browser === undefined && firefox !== undefined) {
        browser = firefox;
    }

    function isUrl(k) {
        if (k.href === undefined || k.href === null) {
            return false;
        }
        return true;
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

    function keydown(evt) {
        if (!evt) evt = event;
        if (evt.ctrlKey && evt.keyCode && evt.keyCode == 222) { //CTRL+²
            let o = document.querySelectorAll("a:hover");
            browser.storage.local.get(['server_url'], function (result) {
                if (o.length === 0 || !isUrl(o[0])) {
                    window.location.href = genURL(result.server_url) + btoa(window.location.href).split("/").join("_");
                } else {
                    window.location.href = genURL(result.server_url) + btoa(
                        o[0].protocol + "//" + o[0].host + o[0].pathname + o[0].search + o[0].hash
                    ).split("/").join("_");
                }
            });
        }
    }
    document.addEventListener("keydown", keydown);
})();