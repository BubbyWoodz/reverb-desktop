/* Reverb first-run setup: save the server URL, then hand off to it. */
(function () {
  "use strict";

  var STORE_PATH = "reverb-settings.dat";
  var SERVER_KEY = "server_url";

  function $(id) {
    return document.getElementById(id);
  }

  function normalizeUrl(raw) {
    var u = (raw || "").trim();
    if (!u) return "";
    if (!/^https?:\/\//i.test(u)) u = "http://" + u;
    return u.replace(/\/+$/, "");
  }

  /* Persistent store: Tauri store plugin when available, localStorage fallback. */
  function makeStore() {
    try {
      if (window.__TAURI__ && window.__TAURI__.core) {
        var invoke = window.__TAURI__.core.invoke;
        return {
          load: function () {
            return invoke("plugin:store|load", {
              path: STORE_PATH,
              options: { autoSave: true },
            }).then(function (rid) {
              return {
                get: function (k) {
                  return invoke("plugin:store|get", { rid: rid, key: k }).then(
                    function (res) {
                      return res && res[1] ? res[0] : undefined;
                    }
                  );
                },
                set: function (k, v) {
                  return invoke("plugin:store|set", {
                    rid: rid,
                    key: k,
                    value: v,
                  }).then(function () {
                    return invoke("plugin:store|save", { rid: rid });
                  });
                },
              };
            });
          },
        };
      }
    } catch (e) {
      /* fall through to localStorage */
    }
    return {
      load: function () {
        return Promise.resolve({
          get: function (k) {
            try {
              var v = localStorage.getItem("reverb:" + k);
              return Promise.resolve(v ? JSON.parse(v) : undefined);
            } catch (e) {
              return Promise.resolve(undefined);
            }
          },
          set: function (k, v) {
            try {
              localStorage.setItem("reverb:" + k, JSON.stringify(v));
            } catch (e) {}
            return Promise.resolve();
          },
        });
      },
    };
  }

  function setStatus(msg, kind) {
    var el = $("status");
    el.textContent = msg;
    el.className = "status " + (kind || "");
  }

  /* Reachability probe: opaque response = reachable, TypeError = not. */
  function probe(url) {
    return fetch(url, { mode: "no-cors", cache: "no-store" }).then(
      function () {
        return true;
      },
      function () {
        return false;
      }
    );
  }

  function go(url) {
    // Remember the local origin so the desktop bridge can navigate back here.
    try {
      localStorage.setItem("reverb:local-origin", window.location.origin);
    } catch (e) {}
    setStatus("Connecting…", "busy");
    window.location.href = url;
  }

  var storePromise = makeStore().load();

  document.addEventListener("DOMContentLoaded", function () {
    var form = $("setup-form");
    var input = $("server-url");
    var testBtn = $("test-btn");
    var connectBtn = $("connect-btn");

    // If a server is already saved, verify it's reachable, then go.
    storePromise.then(function (store) {
      return store.get(SERVER_KEY).then(function (saved) {
        var url = normalizeUrl(saved || "");
        if (!url) return;
        input.value = url;
        setStatus("Connecting to your server…", "busy");
        probe(url).then(function (ok) {
          if (ok) {
            go(url);
          } else {
            setStatus(
              "Couldn't reach that server. Check the address and try again.",
              "err"
            );
          }
        });
      });
    });

    testBtn.addEventListener("click", function () {
      var url = normalizeUrl(input.value);
      if (!url) {
        setStatus("Enter your server address first.", "err");
        return;
      }
      setStatus("Testing…", "busy");
      testBtn.disabled = true;
      probe(url).then(function (ok) {
        testBtn.disabled = false;
        setStatus(
          ok ? "Server reachable." : "Couldn't reach that server.",
          ok ? "ok" : "err"
        );
      });
    });

    form.addEventListener("submit", function (e) {
      e.preventDefault();
      var url = normalizeUrl(input.value);
      if (!url) {
        setStatus("Enter your server address first.", "err");
        return;
      }
      setStatus("Connecting…", "busy");
      connectBtn.disabled = true;
      probe(url).then(function (ok) {
        if (!ok) {
          connectBtn.disabled = false;
          setStatus(
            "Couldn't reach that server. Check the address and try again.",
            "err"
          );
          return;
        }
        storePromise.then(function (store) {
          return store.set(SERVER_KEY, url).then(function () {
            go(url);
          });
        });
      });
    });

    input.focus();
  });
})();
