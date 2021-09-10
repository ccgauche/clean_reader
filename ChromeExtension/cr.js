(function() {
    function isUrl(k) {
        if (k.href === undefined || k.href === null) {
            return false;
        }
        return true;
    }
    function keydown(evt){
        if (!evt) evt = event;
        if (evt.ctrlKey && evt.keyCode && evt.keyCode==222){ //CTRL+²
            let o = document.querySelectorAll("a:hover");
            if (o.length === 0 || !isUrl(o[0])) {
                window.location.href = "http://localhost:8080/r/"+btoa(window.location.href).split("/").join("_");
            } else {
                window.location.href = "http://localhost:8080/r/"+btoa(
                    o[0].protocol+"//"+o[0].host+o[0].pathname+o[0].search+o[0].hash
                ).split("/").join("_");
            }
        }
    }
    document.addEventListener("keydown", keydown);
})();