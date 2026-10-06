/*
 * Reverb desktop bridge — injected into every page (setup + remote server).
 *
 * Provides window.__reverb for native control (media keys, tray) and reports
 * track changes back to Rust for native notifications.
 *
 * Player hooks (grounded in the Reverb webclient):
 *  - Audio elements: #source-0 / #source-1 appended to document.body
 *  - Next/prev/play-pause: .hotkeys buttons (prev, play/pause, next)
 *  - Now-playing metadata: navigator.mediaSession.metadata
 */
(function () {
  "use strict";

  function audioElements() {
    var els = [];
    ["source-0", "source-1"].forEach(function (id) {
      var el = document.getElementById(id);
      if (el) els.push(el);
    });
    // Fallback: any audio element on the page.
    if (!els.length) {
      var any = document.querySelector("audio");
      if (any) els.push(any);
    }
    return els;
  }

  function activeAudio() {
    var els = audioElements();
    if (!els.length) return null;
    var playing = els.find(function (a) {
      return !a.paused;
    });
    if (playing) return playing;
    var withSrc = els.find(function (a) {
      return (a.currentSrc || a.src || "").length > 0;
    });
    return withSrc || els[0];
  }

  function hotkeyButton(index) {
    var bar = document.querySelector(".hotkeys");
    if (!bar) return null;
    var btns = bar.querySelectorAll("button");
    return btns[index] || null;
  }

  function clickHotkey(index) {
    var btn = hotkeyButton(index);
    if (btn) {
      btn.click();
      return true;
    }
    return false;
  }

  var api = {
    ready: true,

    togglePlay: function () {
      // Prefer the app's own control (handles crossfade, queue state, etc).
      if (clickHotkey(1)) return true;
      var a = activeAudio();
      if (!a) return false;
      if (a.paused) {
        a.play().catch(function () {});
      } else {
        a.pause();
      }
      return true;
    },

    play: function () {
      var a = activeAudio();
      if (!a) return false;
      if (a.paused) {
        // Go through app logic when possible so state stays consistent.
        if (!clickHotkey(1)) a.play().catch(function () {});
      }
      return true;
    },

    pause: function () {
      var a = activeAudio();
      if (!a) return false;
      if (!a.paused) {
        if (!clickHotkey(1)) a.pause();
      }
      return true;
    },

    next: function () {
      return clickHotkey(2);
    },

    prev: function () {
      return clickHotkey(0);
    },

    isPlaying: function () {
      var a = activeAudio();
      return !!(a && !a.paused);
    },
  };

  window.__reverb = api;

  /* ---- Track-change detection -> native notifications ---- */
  var lastKey = null;
  var firstRun = true;

  function pollMetadata() {
    try {
      var md =
        navigator.mediaSession && navigator.mediaSession.metadata;
      if (!md || !md.title) return;
      var key = md.title + "|" + (md.artist || "") + "|" + (md.album || "");
      if (key === lastKey) return;
      var isFirst = firstRun;
      lastKey = key;
      firstRun = false;
      if (isFirst) return; // don't notify for the track already playing on load
      if (window.__TAURI__ && window.__TAURI__.core) {
        window.__TAURI__.core
          .invoke("notify_track_change", {
            title: md.title || "",
            artist: md.artist || "",
            album: md.album || "",
          })
          .catch(function () {});
      }
    } catch (e) {
      /* never break the page */
    }
  }

  setInterval(pollMetadata, 1500);

  /* ---- "Change server" from the tray menu ---- */
  try {
    if (window.__TAURI__ && window.__TAURI__.event) {
      window.__TAURI__.event
        .listen("reverb:goto-setup", function () {
          var origin = null;
          try {
            origin = localStorage.getItem("reverb:local-origin");
          } catch (e) {}
          if (origin) {
            window.location.href = origin.replace(/\/+$/, "") + "/index.html";
          }
        })
        .catch(function () {});
    }
  } catch (e) {
    /* never break the page */
  }
})();
