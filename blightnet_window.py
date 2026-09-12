#!/usr/bin/env python3
"""Standalone Blightnet window: a dedicated WebKit browser for the table."""

from __future__ import annotations

import os
import sys

ROOT = os.path.dirname(os.path.abspath(__file__))
ICON = os.path.join(ROOT, "assets", "icon.png")
DATA = os.path.join(os.path.expanduser("~/.local/share"), "blightnet")
CACHE = os.path.join(os.path.expanduser("~/.cache"), "blightnet")

def _software_gl() -> bool:
    backend = (os.environ.get("GDK_BACKEND") or "").lower()
    if backend == "x11":
        return False
    if backend == "wayland":
        return True
    return bool(os.environ.get("WAYLAND_DISPLAY"))


# Forced GPU compositing on native Wayland aborts with protocol error 71.
# X11/XWayland can use the GPU; that is what keeps the table smooth.
if _software_gl():
    os.environ.setdefault("WEBKIT_DISABLE_COMPOSITING_MODE", "1")
else:
    os.environ.pop("WEBKIT_DISABLE_COMPOSITING_MODE", None)
    os.environ.setdefault("WEBKIT_DISABLE_DMABUF_RENDERER", "1")
    os.environ.setdefault("WEBKIT_FORCE_COMPOSITING_MODE", "1")


def _ensure_gst_plugin() -> None:
    """WebKit looks up autoaudiosink (gst-plugins-good). Provide pipewire/alsa."""
    gst_dir = os.path.join(ROOT, "gst")
    so = os.path.join(gst_dir, "libgstblighnetaudio.so")
    src = os.path.join(ROOT, "tools", "gst_autoaudiosink.c")
    os.makedirs(gst_dir, exist_ok=True)
    stale = os.path.isfile(so) and os.path.isfile(src) and os.path.getmtime(src) > os.path.getmtime(so)
    if (not os.path.isfile(so) or stale) and os.path.isfile(src):
        import subprocess

        try:
            flags = subprocess.check_output(
                ["pkg-config", "--cflags", "--libs", "gstreamer-1.0"],
                text=True,
            ).split()
            subprocess.check_call(["gcc", "-shared", "-fPIC", "-o", so, src, *flags])
            print("Blightnet audio plugin built.", flush=True)
        except Exception as exc:
            print(f"Blightnet audio plugin: compile skipped ({exc}).", flush=True)
    if os.path.isdir(gst_dir):
        old = os.environ.get("GST_PLUGIN_PATH", "")
        parts = [p for p in old.split(":") if p]
        if gst_dir not in parts:
            os.environ["GST_PLUGIN_PATH"] = gst_dir + ((":" + old) if old else "")


_ensure_gst_plugin()





def _gtk_webkit():
    import gi

    # Gdk 4 is the default on this machine. Pin 3.0 before Gtk 3 / WebKitGTK.
    gi.require_version("Gdk", "3.0")
    gi.require_version("Gtk", "3.0")
    try:
        gi.require_version("WebKit2", "4.1")
    except ValueError:
        gi.require_version("WebKit2", "4.0")
    from gi.repository import Gdk, Gio, GLib, Gtk, WebKit2

    return Gdk, Gio, GLib, Gtk, WebKit2


def _is_wayland(Gdk) -> bool:
    try:
        display = Gdk.Display.get_default()
        name = (display.get_name() if display else "") or ""
        return "wayland" in name.lower() or bool(os.environ.get("WAYLAND_DISPLAY"))
    except Exception:
        return bool(os.environ.get("WAYLAND_DISPLAY"))


def _set_setting(settings, name, value) -> None:
    try:
        settings.set_property(name, value)
    except Exception:
        pass


def _fastest_monitor(Gdk):
    display = Gdk.Display.get_default()
    if display is None:
        return -1, None, 0
    try:
        count = display.get_n_monitors()
    except Exception:
        return -1, None, 0
    best = None
    best_i = -1
    best_hz = -1
    best_area = -1
    for i in range(count):
        try:
            mon = display.get_monitor(i)
            hz = int(mon.get_refresh_rate() or 0)
            geo = mon.get_geometry()
            area = int(geo.width) * int(geo.height) if geo else 0
        except Exception:
            continue
        if best is None or (hz, area) > (best_hz, best_area):
            best_hz = hz
            best_area = area
            best = mon
            best_i = i
    return best_i, best, best_hz


def _fit_and_fullscreen(win, Gdk) -> None:
    """Fill the detected monitor and go fullscreen on boot."""
    idx, mon, hz = _fastest_monitor(Gdk)
    if mon is None:
        try:
            win.fullscreen()
        except Exception:
            pass
        return
    try:
        geo = mon.get_geometry()
    except Exception:
        geo = None
    if geo is not None and geo.width > 0:
        try:
            win.move(geo.x, geo.y)
            win.resize(geo.width, geo.height)
        except Exception:
            pass
        print(
            f"Blightnet window: {hz / 1000:.1f} Hz {geo.width}x{geo.height} at {geo.x},{geo.y}",
            flush=True,
        )
    try:
        screen = win.get_screen()
        if idx >= 0 and screen is not None:
            win.fullscreen_on_monitor(screen, idx)
        else:
            win.fullscreen()
    except Exception:
        try:
            win.fullscreen()
        except Exception:
            pass
    print("Blightnet window: fullscreen", flush=True)


def _run_js(view, script: str) -> None:
    try:
        view.run_javascript(script, None, None, None)
    except TypeError:
        try:
            view.run_javascript(script)
        except Exception:
            pass
    except Exception:
        pass


def _uri_of(decision) -> str:
    for getter in (
        lambda: decision.get_navigation_action().get_request().get_uri(),
        lambda: decision.get_request().get_uri(),
    ):
        try:
            uri = getter()
            if uri:
                return uri
        except Exception:
            pass
    return ""


def open_window(url: str, on_close=None) -> bool:
    """Open the native Blightnet browser. Returns False if WebKit is unavailable."""
    try:
        Gdk, Gio, GLib, Gtk, WebKit2 = _gtk_webkit()
    except Exception as exc:
        print(f"Native window unavailable ({exc}).", flush=True)
        return False

    try:
        Gdk.set_program_class("Blightnet")
    except Exception:
        pass

    app = Gtk.Application(
        application_id="local.blightnet.shell",
        flags=Gio.ApplicationFlags.NON_UNIQUE,
    )
    state = {"shown": False}

    def on_activate(application):
        try:
            css = Gtk.CssProvider()
            css.load_from_data(
                b"""
                window { background-color: #0a0010; }
                headerbar {
                    background-color: #140010;
                    background-image: none;
                    border-bottom: 1px solid #ff2d6a;
                    min-height: 36px;
                    color: #ff2d6a;
                }
                headerbar label { color: #ff2d6a; }
                button {
                    background: #1a0014;
                    background-image: none;
                    color: #ff2d6a;
                    border: 1px solid #ff2d6a;
                    border-radius: 0;
                    min-height: 26px;
                }
                button:hover { color: #00f0ff; }
                entry {
                    background: #050008;
                    background-image: none;
                    color: #00f0ff;
                    border: 1px solid #ff2d6a;
                    border-radius: 0;
                    caret-color: #ff2d6a;
                    min-height: 26px;
                }
                """
            )
            screen = Gdk.Screen.get_default()
            if screen is not None:
                Gtk.StyleContext.add_provider_for_screen(
                    screen, css, Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION
                )
        except Exception as exc:
            print(f"Blightnet window: theme skipped ({exc}).", flush=True)

        win = Gtk.ApplicationWindow(application=application)
        win.set_title("Blightnet")
        win.set_default_size(1440, 900)
        try:
            import warnings

            with warnings.catch_warnings():
                warnings.simplefilter("ignore", DeprecationWarning)
                win.set_wmclass("Blightnet", "Blightnet")
        except Exception:
            pass
        try:
            win.set_position(Gtk.WindowPosition.NONE)
        except Exception:
            pass
        if os.path.isfile(ICON):
            try:
                win.set_icon_from_file(ICON)
            except Exception:
                pass

        header = Gtk.HeaderBar()
        header.set_show_close_button(True)
        header.set_title("BLIGHTNET")
        try:
            header.set_subtitle("LOCAL SHELL")
        except Exception:
            pass
        win.set_titlebar(header)

        back = Gtk.Button(label="◀")
        fwd = Gtk.Button(label="▶")
        reload_btn = Gtk.Button(label="↻")
        home = Gtk.Button(label="⌂")
        for b in (back, fwd, reload_btn, home):
            header.pack_start(b)

        omnibox = Gtk.Entry()
        omnibox.set_text(url)
        omnibox.set_placeholder_text("blightnet://start")
        omnibox.set_hexpand(True)
        box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=4)
        box.pack_start(omnibox, True, True, 0)
        header.pack_end(box)

        settings = WebKit2.Settings()
        _set_setting(settings, "enable-javascript", True)
        gpu = not _software_gl()
        _set_setting(settings, "enable-webgl", gpu)
        _set_setting(settings, "enable-accelerated-2d-canvas", gpu)
        _set_setting(settings, "enable-2d-canvas-acceleration", gpu)
        _set_setting(settings, "enable-media", True)
        _set_setting(settings, "enable-webaudio", True)
        _set_setting(settings, "enable-media-stream", True)
        _set_setting(settings, "enable-smooth-scrolling", False)
        _set_setting(settings, "media-playback-requires-user-gesture", False)
        _set_setting(settings, "media-playback-allows-inline", True)
        _set_setting(settings, "auto-load-images", True)
        _set_setting(settings, "enable-developer-extras", False)
        _set_setting(settings, "enable-page-cache", True)
        _set_setting(settings, "enable-html5-database", True)
        _set_setting(settings, "enable-html5-local-storage", True)
        try:
            policy = (
                WebKit2.HardwareAccelerationPolicy.ALWAYS
                if gpu
                else WebKit2.HardwareAccelerationPolicy.NEVER
            )
        except Exception:
            policy = WebKit2.HardwareAccelerationPolicy.NEVER
        _set_setting(settings, "hardware-acceleration-policy", policy)

        def prep_context(ctx) -> None:
            gst_dir = os.path.join(ROOT, "gst")
            try:
                ctx.set_sandbox_enabled(False)
            except Exception:
                pass
            for path in (gst_dir, DATA, CACHE, ROOT):
                try:
                    ctx.add_path_to_sandbox(path, True)
                except Exception:
                    pass

        view = None
        try:
            os.makedirs(DATA, exist_ok=True)
            os.makedirs(CACHE, exist_ok=True)
            mgr = WebKit2.WebsiteDataManager(
                base_data_directory=DATA,
                base_cache_directory=CACHE,
            )
            ctx = WebKit2.WebContext.new_with_website_data_manager(mgr)
            prep_context(ctx)
            view = WebKit2.WebView.new_with_context(ctx)
            view.set_settings(settings)
        except Exception as exc:
            print(f"Blightnet window: default profile ({exc}).", flush=True)
            try:
                prep_context(WebKit2.WebContext.get_default())
            except Exception:
                pass
            view = WebKit2.WebView()
            view.set_settings(settings)

        def sync_nav(*_a):
            try:
                back.set_sensitive(view.can_go_back())
                fwd.set_sensitive(view.can_go_forward())
            except Exception:
                pass
            uri = view.get_uri() or ""
            if uri and omnibox.get_text() != uri and not omnibox.has_focus():
                omnibox.set_text(uri)

        def go_home(*_a):
            view.load_uri(url)

        def submit(*_a):
            raw = (omnibox.get_text() or "").strip()
            if not raw:
                return
            low = raw.lower()
            if low.startswith("blighnet://") or low.startswith("blightnet://"):
                rest = raw.split("://", 1)[-1]
                if rest in ("", "start", "/"):
                    view.load_uri(url)
                elif rest.startswith("hearthsong") or rest.startswith("table"):
                    here = view.get_uri() or ""
                    if here.startswith(url):
                        _run_js(
                            view,
                            "document.getElementById('launch-hearthsong')&&document.getElementById('launch-hearthsong').click()",
                        )
                    else:
                        view.load_uri(url.rstrip("/") + "/#table")
                else:
                    view.load_uri("http://" + rest)
                return
            if "://" not in raw:
                raw = "http://" + raw
            view.load_uri(raw)

        def on_key(widget, event):
            name = Gdk.keyval_name(event.keyval) or ""
            ctrl = bool(event.state & Gdk.ModifierType.CONTROL_MASK)
            if name == "F11":
                if win.is_fullscreen():
                    win.unfullscreen()
                else:
                    win.fullscreen()
                return True
            if name == "F5" or (ctrl and name.lower() == "r"):
                view.reload()
                return True
            if ctrl and name.lower() == "l":
                omnibox.grab_focus()
                omnibox.select_region(0, -1)
                return True
            if ctrl and name == "Left":
                view.go_back()
                return True
            if ctrl and name == "Right":
                view.go_forward()
                return True
            return False

        def on_crash(v, *_a):
            print("Blightnet window: page process restarted.", flush=True)
            try:
                v.load_uri(url)
            except Exception:
                try:
                    v.reload()
                except Exception:
                    pass

        def on_fail(v, _ev, uri, err):
            print(f"Blightnet window: load failed {uri} ({err}).", flush=True)
            return False

        def on_create(_v, navigation_action):
            # Keep window.open / target=_blank inside Blightnet. Never hand
            # those URLs to Firefox or Brave.
            try:
                req = navigation_action.get_request()
                uri = req.get_uri() if req else ""
            except Exception:
                uri = ""
            if uri:
                GLib.idle_add(view.load_uri, uri)
            return None

        def on_policy(v, decision, decision_type):
            uri = _uri_of(decision)
            if uri.startswith("blighnet://quit") or uri.startswith("blighnet://exit") or uri.startswith("blightnet://quit") or uri.startswith("blightnet://exit"):
                try:
                    decision.ignore()
                except Exception:
                    pass
                application.quit()
                return True
            if decision_type == WebKit2.PolicyDecisionType.NEW_WINDOW_ACTION:
                if uri:
                    v.load_uri(uri)
                try:
                    decision.ignore()
                except Exception:
                    pass
                return True
            return False

        def place_fast():
            try:
                _fit_and_fullscreen(win, Gdk)
            except Exception as exc:
                print(f"Blightnet window: fullscreen skipped ({exc}).", flush=True)
            return False

        def on_map(*_a):
            if not state["shown"]:
                state["shown"] = True
                print("Blightnet window is on screen.", flush=True)
                place_fast()
                GLib.timeout_add(400, place_fast)
            return False

        back.connect("clicked", lambda *_: view.go_back())
        fwd.connect("clicked", lambda *_: view.go_forward())
        reload_btn.connect("clicked", lambda *_: view.reload())
        home.connect("clicked", go_home)
        omnibox.connect("activate", submit)
        view.connect("load-changed", sync_nav)
        view.connect("create", on_create)
        try:
            view.connect("decide-policy", on_policy)
        except Exception:
            pass
        try:
            view.connect("web-process-terminated", on_crash)
        except Exception:
            pass
        try:
            view.connect("load-failed", on_fail)
        except Exception:
            pass
        win.connect("key-press-event", on_key)
        win.connect("map-event", on_map)

        def closed(*_a):
            if on_close:
                try:
                    on_close()
                except Exception:
                    pass
            application.quit()

        win.connect("destroy", closed)
        win.add(view)
        win.show_all()
        try:
            _fit_and_fullscreen(win, Gdk)
        except Exception as exc:
            print(f"Blightnet window: fullscreen skipped ({exc}).", flush=True)
        try:
            win.set_keep_above(True)
            win.present()
            try:
                win.get_window().raise_()
            except Exception:
                pass
            GLib.timeout_add(1200, lambda: win.set_keep_above(False) or False)
        except Exception:
            try:
                win.present()
            except Exception:
                pass
        view.load_uri(url)
        application._win = win
        application._view = view

    app.connect("activate", on_activate)
    try:
        app.run([])
    except Exception as exc:
        print(f"Blightnet window: GTK run failed ({exc}).", flush=True)
        return False
    return True


if __name__ == "__main__":
    target = sys.argv[1] if len(sys.argv) > 1 else "http://127.0.0.1:8765/"
    ok = open_window(target)
    sys.exit(0 if ok else 1)
