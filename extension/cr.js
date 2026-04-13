(function () {
    let browser = undefined;
    if (browser === undefined && chrome !== undefined) {
        browser = chrome;
    }
    if (browser === undefined && firefox !== undefined) {
        browser = firefox;
    }

    const DEFAULT_SHORTCUT = {
        ctrl: false,
        alt: true,
        shift: false,
        meta: false,
        code: "KeyR"
    };

    let shortcut = DEFAULT_SHORTCUT;
    let serverUrl = undefined;

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

    function encodeTarget(targetUrl) {
        return btoa(targetUrl).split("/").join("_");
    }

    function openCleanRead(targetUrl) {
        window.location.href = genURL(serverUrl) + encodeTarget(targetUrl);
    }

    function cleanReadCurrent() {
        const hovered = document.querySelectorAll("a:hover");
        if (hovered.length === 0 || !isUrl(hovered[0])) {
            openCleanRead(window.location.href);
        } else {
            const a = hovered[0];
            openCleanRead(a.protocol + "//" + a.host + a.pathname + a.search + a.hash);
        }
    }

    function shortcutMatches(evt, s) {
        return evt.code === s.code &&
            !!evt.ctrlKey === !!s.ctrl &&
            !!evt.altKey === !!s.alt &&
            !!evt.shiftKey === !!s.shift &&
            !!evt.metaKey === !!s.meta;
    }

    function keydown(evt) {
        if (!evt) evt = event;
        if (shortcutMatches(evt, shortcut)) {
            evt.preventDefault();
            cleanReadCurrent();
        }
    }
    document.addEventListener("keydown", keydown);

    function injectButton() {
        if (document.getElementById("__clean_reader_btn")) return;
        if (!document.body) return;
        const btn = document.createElement("div");
        btn.id = "__clean_reader_btn";
        btn.textContent = "CR";
        btn.title = "Open in Clean Reader";
        btn.style.cssText = [
            "position:fixed",
            "top:12px",
            "right:12px",
            "z-index:2147483647",
            "width:36px",
            "height:36px",
            "border-radius:50%",
            "background:#111",
            "color:#fff",
            "font:bold 13px/36px -apple-system,Segoe UI,Arial,sans-serif",
            "text-align:center",
            "cursor:pointer",
            "box-shadow:0 2px 6px rgba(0,0,0,0.3)",
            "opacity:0.85",
            "user-select:none"
        ].join(";");
        btn.addEventListener("mouseenter", () => { btn.style.opacity = "1"; });
        btn.addEventListener("mouseleave", () => { btn.style.opacity = "0.85"; });
        btn.addEventListener("click", (e) => {
            e.preventDefault();
            e.stopPropagation();
            openCleanRead(window.location.href);
        });
        document.body.appendChild(btn);
    }

    if (document.readyState === "loading") {
        document.addEventListener("DOMContentLoaded", injectButton);
    } else {
        injectButton();
    }

    browser.storage.local.get(["server_url", "shortcut"], function (result) {
        serverUrl = result.server_url;
        if (result.shortcut) shortcut = result.shortcut;
    });

    browser.storage.onChanged.addListener(function (changes, _namespace) {
        if (changes.server_url) serverUrl = changes.server_url.newValue;
        if (changes.shortcut && changes.shortcut.newValue) shortcut = changes.shortcut.newValue;
    });
})();
