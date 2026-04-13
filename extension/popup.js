let svurl = document.getElementById("svurl");
let shortcutInput = document.getElementById("shortcut");
let resetBtn = document.getElementById("reset_shortcut");

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

function formatShortcut(s) {
    const parts = [];
    if (s.ctrl) parts.push("Ctrl");
    if (s.alt) parts.push("Alt");
    if (s.shift) parts.push("Shift");
    if (s.meta) parts.push("Meta");
    parts.push(s.code.replace(/^Key/, "").replace(/^Digit/, ""));
    return parts.join(" + ");
}

svurl.addEventListener("keyup", () => {
    let v = genURL(svurl.value);
    svurl.value = v;
    browser.storage.local.set({ "server_url": v });
});

shortcutInput.addEventListener("keydown", (e) => {
    e.preventDefault();
    // Ignore bare modifier presses — wait for the actual key.
    if (["Control", "Alt", "Shift", "Meta"].includes(e.key)) return;
    const next = {
        ctrl: e.ctrlKey,
        alt: e.altKey,
        shift: e.shiftKey,
        meta: e.metaKey,
        code: e.code
    };
    browser.storage.local.set({ "shortcut": next });
    shortcutInput.value = formatShortcut(next);
});

resetBtn.addEventListener("click", () => {
    browser.storage.local.set({ "shortcut": DEFAULT_SHORTCUT });
    shortcutInput.value = formatShortcut(DEFAULT_SHORTCUT);
});

browser.storage.local.get(["server_url", "shortcut"], ({ server_url, shortcut }) => {
    svurl.value = server_url || "";
    shortcutInput.value = formatShortcut(shortcut || DEFAULT_SHORTCUT);
});

browser.storage.onChanged.addListener((changes) => {
    if (changes.server_url) {
        svurl.value = genURL(changes.server_url.newValue);
    }
    if (changes.shortcut && changes.shortcut.newValue) {
        shortcutInput.value = formatShortcut(changes.shortcut.newValue);
    }
});
