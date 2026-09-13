#!/usr/bin/env python3
"""Open Blightnet in a browser. Threaded so every sound can load at once."""

from __future__ import annotations

import argparse
import base64
import functools
import hashlib
import http.server
import json
import os
import re
import shutil
import signal
import socket
import struct
import subprocess
import sys
import threading
import time
import urllib.error
import urllib.parse
import urllib.request
import uuid
import webbrowser

ROOT = os.path.dirname(os.path.abspath(__file__))
PORT_DEFAULT = 8765
WS_GUID = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11"
MAX_UPLOAD = 40 * 1024 * 1024
MAX_WS = 4 * 1024 * 1024
UPLOAD_DIR = os.path.join(ROOT, "uploads")
MANIFEST_PATH = os.path.join(UPLOAD_DIR, "manifest.json")
AUDIO_EXT = {".ogg", ".oga", ".mp3", ".wav", ".m4a", ".flac", ".webm", ".aac"}
IMAGE_EXT = {".jpg", ".jpeg", ".png", ".webp", ".gif"}
WHISPER_CMD = re.compile(r"^/(?:w|whisper|msg|tell)(?:\s+|$)(.*)$", re.I)
PIC_MAX = 120000


def _clean_pic(raw) -> str:
    s = str(raw or "")
    if not s.startswith("data:image/") or "base64," not in s:
        return ""
    if len(s) > PIC_MAX:
        return ""
    return s

_DATA_NEW = os.path.join(os.path.expanduser("~/.local/share"), "blightnet")
_DATA_OLD = os.path.join(os.path.expanduser("~/.local/share"), "blighnet")
os.makedirs(_DATA_NEW, exist_ok=True)
if not os.path.exists(os.path.join(_DATA_NEW, "display.json")):
    _old_pref = os.path.join(_DATA_OLD, "display.json")
    if os.path.exists(_old_pref):
        try:
            shutil.copy2(_old_pref, os.path.join(_DATA_NEW, "display.json"))
        except OSError:
            pass
CHROME_PROFILE = os.path.join(_DATA_NEW, "gpu-profile")
DISPLAY_PREF = os.path.join(_DATA_NEW, "display.json")

_window_proc = None
_httpd = None


def request_shutdown() -> None:
    def go() -> None:
        time.sleep(0.05)
        proc = _window_proc
        if proc is not None and proc.poll() is None:
            try:
                proc.terminate()
            except Exception:
                pass
            try:
                proc.wait(timeout=2)
            except Exception:
                try:
                    proc.kill()
                except Exception:
                    pass
        try:
            stop_relay()
        except Exception:
            pass
        httpd = _httpd
        if httpd is not None:
            try:
                httpd.shutdown()
            except Exception:
                pass
        try:
            os.kill(os.getpid(), signal.SIGTERM)
        except Exception:
            os._exit(0)

    threading.Thread(target=go, daemon=True).start()


def _outbound_lan() -> str:
    try:
        s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        s.connect(("1.1.1.1", 80))
        ip = s.getsockname()[0]
        s.close()
        if ip and not ip.startswith("127."):
            return ip
    except Exception:
        pass
    return ""


def lan_ips() -> list[str]:
    found: list[str] = []

    def add(ip: str) -> None:
        if ip and "." in ip and not ip.startswith("127.") and ip not in found:
            found.append(ip)

    add(_outbound_lan())
    try:
        out = subprocess.check_output(["hostname", "-I"], text=True, stderr=subprocess.DEVNULL)
        for ip in out.split():
            add(ip)
    except Exception:
        pass
    try:
        for info in socket.getaddrinfo(socket.gethostname(), None, socket.AF_INET):
            add(info[4][0])
    except Exception:
        pass
    return found


def ipv6_ips() -> list[str]:
    found: list[str] = []

    def add(ip: str) -> None:
        ip = (ip or "").split("%")[0]
        if not ip or ip in found:
            return
        if ip in ("::1",) or ip.startswith("fe80:") or ip.startswith("fd") or ip.startswith("fc"):
            return
        if ":" not in ip:
            return
        found.append(ip)

    try:
        out = subprocess.check_output(
            ["ip", "-6", "-o", "addr", "show", "scope", "global"],
            text=True,
            stderr=subprocess.DEVNULL,
        )
        for line in out.splitlines():
            parts = line.split()
            if "inet6" in parts:
                add(parts[parts.index("inet6") + 1].split("/")[0])
    except Exception:
        pass
    try:
        out = subprocess.check_output(["hostname", "-I"], text=True, stderr=subprocess.DEVNULL)
        for ip in out.split():
            add(ip)
    except Exception:
        pass
    try:
        for info in socket.getaddrinfo(socket.gethostname(), None, socket.AF_INET6):
            add(info[4][0])
    except Exception:
        pass
    return found


NET = {"wan": "", "wan6": "", "upnp": False, "port": PORT_DEFAULT, "relay": ""}
_relay_proc = None
_relay_lock = threading.Lock()


def stun_wan_ip() -> str:
    magic = 0x2112A442
    tid = os.urandom(12)
    pkt = struct.pack("!HHI", 0x0001, 0, magic) + tid
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.settimeout(1.4)
    try:
        for host in ("stun.l.google.com", "stun1.l.google.com"):
            try:
                sock.sendto(pkt, (host, 19302))
                data, _ = sock.recvfrom(2048)
            except Exception:
                continue
            if len(data) < 20:
                continue
            length = struct.unpack("!H", data[2:4])[0]
            off = 20
            end = min(len(data), 20 + length)
            while off + 4 <= end:
                atype, alen = struct.unpack("!HH", data[off : off + 4])
                val = data[off + 4 : off + 4 + alen]
                off += 4 + alen
                if alen % 4:
                    off += 4 - (alen % 4)
                if atype not in (0x0020, 0x0001) or len(val) < 8 or val[1] != 1:
                    continue
                ipb = val[4:8]
                if atype == 0x0020:
                    ipb = bytes(b ^ ((magic >> (24 - 8 * i)) & 0xFF) for i, b in enumerate(ipb))
                return ".".join(str(b) for b in ipb)
    finally:
        sock.close()
    return ""


def _http_wan_ip() -> str:
    for url in (
        "https://api.ipify.org",
        "https://icanhazip.com",
        "https://ifconfig.me/ip",
    ):
        try:
            import urllib.request

            with urllib.request.urlopen(url, timeout=2.2) as resp:
                ip = resp.read().decode("ascii", "ignore").strip()
            if ip.count(".") == 3 and all(p.isdigit() and int(p) < 256 for p in ip.split(".")):
                return ip
        except Exception:
            continue
    return ""


def _default_gateway() -> str:
    try:
        with open("/proc/net/route", encoding="utf-8") as f:
            for line in f:
                parts = line.split()
                if len(parts) > 2 and parts[1] == "00000000" and parts[0] != "Iface":
                    return socket.inet_ntoa(struct.pack("<I", int(parts[2], 16)))
    except Exception:
        pass
    return ""


def _natpmp_map(port: int) -> str:
    gw = _default_gateway()
    if not gw:
        return ""
    pkt = struct.pack("!BBHHHI", 0, 2, 0, port, port, 7200)
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.settimeout(1.4)
    try:
        sock.sendto(pkt, (gw, 5351))
        data, _ = sock.recvfrom(32)
        if len(data) >= 16:
            _ver, _op, result, _epoch, _priv, _pub, _life = struct.unpack("!BBHIHHI", data[:16])
            if result == 0:
                return stun_wan_ip() or _http_wan_ip()
    except Exception:
        return ""
    finally:
        sock.close()
    return ""


def _upnp_map(port: int) -> str:
    search = (
        "M-SEARCH * HTTP/1.1\r\n"
        "HOST: 239.255.255.250:1900\r\n"
        'MAN: "ssdp:discover"\r\n'
        "MX: 2\r\n"
        "ST: urn:schemas-upnp-org:device:InternetGatewayDevice:1\r\n"
        "\r\n"
    ).encode()
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.settimeout(1.6)
    loc = ""
    try:
        sock.sendto(search, ("239.255.255.250", 1900))
        data, _ = sock.recvfrom(4096)
        text = data.decode("utf-8", "ignore")
        for line in text.splitlines():
            if line.lower().startswith("location:"):
                loc = line.split(":", 1)[1].strip()
                break
    except Exception:
        loc = ""
    finally:
        sock.close()
    if not loc:
        return ""
    try:
        import urllib.request
        import xml.etree.ElementTree as ET

        xml_text = urllib.request.urlopen(loc, timeout=2).read()
        tree = ET.fromstring(xml_text)
        ns = {"d": "urn:schemas-upnp-org:device-1-0"}
        ctrl = ""
        for svc in tree.findall(".//d:service", ns) or tree.findall(".//{urn:schemas-upnp-org:device-1-0}service"):
            st = (svc.findtext("{urn:schemas-upnp-org:device-1-0}serviceType") or svc.findtext("serviceType") or "")
            if "WANIPConnection" in st or "WANPPPConnection" in st:
                ctrl = svc.findtext("{urn:schemas-upnp-org:device-1-0}controlURL") or svc.findtext("controlURL") or ""
                svc_type = st
                break
        else:
            return ""
        if not ctrl:
            return ""
        base = loc.rsplit("/", 1)[0]
        if ctrl.startswith("/"):
            host = urllib.parse.urlparse(loc)
            action = f"{host.scheme}://{host.netloc}{ctrl}"
        elif ctrl.startswith("http"):
            action = ctrl
        else:
            action = base + "/" + ctrl
        lan = _outbound_lan() or (lan_ips() or ["127.0.0.1"])[0]
        body = f"""<?xml version="1.0"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/" s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/">
<s:Body><u:AddPortMapping xmlns:u="{svc_type}">
<NewRemoteHost></NewRemoteHost>
<NewExternalPort>{port}</NewExternalPort>
<NewProtocol>TCP</NewProtocol>
<NewInternalPort>{port}</NewInternalPort>
<NewInternalClient>{lan}</NewInternalClient>
<NewEnabled>1</NewEnabled>
<NewPortMappingDescription>Blightnet</NewPortMappingDescription>
<NewLeaseDuration>86400</NewLeaseDuration>
</u:AddPortMapping></s:Body></s:Envelope>"""
        req = urllib.request.Request(
            action,
            data=body.encode(),
            headers={
                "Content-Type": 'text/xml; charset="utf-8"',
                "SOAPAction": f'"{svc_type}#AddPortMapping"',
            },
            method="POST",
        )
        urllib.request.urlopen(req, timeout=2.5).read()
        NET["upnp"] = True
        return stun_wan_ip() or _http_wan_ip()
    except Exception:
        return ""


def _is_cgnat(ip: str) -> bool:
    parts = (ip or "").split(".")
    if len(parts) != 4:
        return False
    try:
        a, b = int(parts[0]), int(parts[1])
    except ValueError:
        return False
    return a == 100 and 64 <= b <= 127


def _pick_relay_url(text: str) -> str:
    for m in re.finditer(r"https?://[a-zA-Z0-9][a-zA-Z0-9.-]+\.[a-zA-Z]{2,}", text or ""):
        url = m.group(0).rstrip("/.,)")
        if url.startswith("http://"):
            url = "https://" + url[7:]
        host = (urllib.parse.urlparse(url).hostname or "").lower()
        if not host or host in ("github.com", "localhost", "127.0.0.1"):
            continue
        if host.endswith(".github.com") or host.endswith(".google.com"):
            continue
        return url
    return ""


def stop_relay() -> None:
    global _relay_proc
    with _relay_lock:
        proc = _relay_proc
        _relay_proc = None
    if not proc:
        return
    if proc.poll() is None:
        try:
            proc.terminate()
        except Exception:
            pass
        try:
            proc.wait(timeout=2)
        except Exception:
            try:
                proc.kill()
            except Exception:
                pass
    NET["relay"] = ""


def _spawn_relay(cmd: list[str], port: int) -> str:
    global _relay_proc
    try:
        proc = subprocess.Popen(
            cmd,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            stdin=subprocess.DEVNULL,
        )
    except Exception:
        return ""
    buf = []
    deadline = time.time() + 18

    def pump() -> None:
        try:
            while True:
                chunk = proc.stdout.read(256)
                if not chunk:
                    break
                buf.append(chunk.decode("utf-8", "ignore"))
                url = _pick_relay_url("".join(buf[-40:]))
                if url and not NET.get("relay"):
                    NET["relay"] = url.rstrip("/")
                    print(f"Internet table (relay) → {NET['relay']}", flush=True)
        except Exception:
            pass

    threading.Thread(target=pump, daemon=True, name="blightnet-relay-log").start()
    while time.time() < deadline:
        if NET.get("relay"):
            with _relay_lock:
                old = _relay_proc
                _relay_proc = proc
            if old and old is not proc and old.poll() is None:
                try:
                    old.kill()
                except Exception:
                    pass
            return NET["relay"]
        if proc.poll() is not None:
            return ""
        time.sleep(0.2)
    if proc.poll() is None and NET.get("relay"):
        with _relay_lock:
            _relay_proc = proc
        return NET["relay"]
    if proc.poll() is None:
        try:
            proc.kill()
        except Exception:
            pass
    return ""


def start_relay(port: int) -> None:
    if NET.get("relay"):
        return
    ssh = shutil.which("ssh")
    cloud = shutil.which("cloudflared")
    local = f"127.0.0.1:{port}"
    attempts: list[list[str]] = []
    if cloud:
        attempts.append(
            [cloud, "tunnel", "--no-autoupdate", "--url", f"http://{local}"]
        )
    if ssh:
        ssh_base = [
            ssh,
            "-T",
            "-o",
            "StrictHostKeyChecking=accept-new",
            "-o",
            "ServerAliveInterval=30",
            "-o",
            "ExitOnForwardFailure=yes",
        ]
        attempts.append(ssh_base + ["-p", "443", "-R", f"0:{local}", "a.pinggy.io"])
        attempts.append(ssh_base + ["-R", f"80:{local}", "nokey@localhost.run"])
        attempts.append(ssh_base + ["-R", f"80:{local}", "serveo.net"])
    for cmd in attempts:
        print("Internet table: opening a path friends can reach…", flush=True)
        url = _spawn_relay(cmd, port)
        if url:
            return
        stop_relay()
    if not NET.get("relay"):
        print("Internet table: no automatic tunnel. LAN still works. Copy address after Host.", flush=True)


def punch_internet(port: int) -> None:
    NET["port"] = port
    wan6 = (ipv6_ips() or [""])[0]
    if wan6:
        NET["wan6"] = wan6
        print(f"Internet table (IPv6) → http://[{wan6}]:{port}/", flush=True)
    wan = ""
    try:
        wan = _upnp_map(port) or _natpmp_map(port) or stun_wan_ip() or _http_wan_ip()
    except Exception:
        wan = stun_wan_ip() or _http_wan_ip()
    if wan and _is_cgnat(wan):
        print(f"Public IPv4 {wan} is carrier NAT — friends cannot dial it directly.", flush=True)
        wan = ""
    if wan:
        NET["wan"] = wan
        mapped = " (router opened)" if NET["upnp"] else ""
        print(f"Internet table (IPv4) → http://{wan}:{port}/{mapped}", flush=True)
    start_relay(port)
    if not NET.get("relay") and not wan and not wan6:
        print("Internet table: no public address yet. Join still works on the LAN.", flush=True)


def load_manifest() -> list:
    try:
        with open(MANIFEST_PATH, "r", encoding="utf-8") as f:
            data = json.load(f)
        if isinstance(data, list):
            return data
    except Exception:
        pass
    return []


def save_manifest(rows: list) -> None:
    os.makedirs(UPLOAD_DIR, exist_ok=True)
    tmp = MANIFEST_PATH + ".tmp"
    with open(tmp, "w", encoding="utf-8") as f:
        json.dump(rows, f)
    os.replace(tmp, MANIFEST_PATH)


def _fold_name(value: str) -> str:
    return " ".join((value or "").strip().lower().split())


def _strip_name_prefix(body: str, name: str) -> str:
    body = (body or "").strip()
    if not body:
        return body
    if body[:1] in "\"'":
        q = body[0]
        end = body.find(q, 1)
        if end != -1 and _fold_name(body[1:end]) == _fold_name(name):
            return body[end + 1 :].strip()
    n = (name or "").strip()
    if n and _fold_name(body).startswith(_fold_name(n)):
        cut = len(n)
        if cut <= len(body) and (cut == len(body) or body[cut].isspace()):
            # raw prefix may differ in case/spacing; walk original by word if needed
            if body.lower().startswith(n.lower()) and (len(body) == len(n) or body[len(n) : len(n) + 1].isspace()):
                return body[len(n) :].strip()
    return body


def _whisper_error(kind: str, hint: str = "") -> str:
    who = (hint or "").strip() or "that name"
    if kind == "self":
        return "Whisper another player, not yourself."
    if kind == "ambiguous":
        return f"Several players match {who}. Use the full handle."
    if kind == "usage":
        return "Whisper with /w Name then your message, or click a name above."
    if kind == "empty":
        return "Type a whisper after the name."
    return f"No one named {who} is at the table."


def _split_whisper(text: str) -> tuple[bool, str]:
    match = WHISPER_CMD.match(text or "")
    if not match:
        return False, text
    return True, (match.group(1) or "").strip()


def _resolve_whisper_rest(rest: str, hub: "Hub", self_id: str) -> tuple["WSClient | None", str, str, str]:
    rest = (rest or "").strip()
    if not rest:
        return None, "", "usage", ""
    if rest[:1] in "\"'":
        q = rest[0]
        end = rest.find(q, 1)
        if end < 0:
            return None, "", "usage", ""
        name = rest[1:end].strip()
        body = rest[end + 1 :].strip()
        target, err = hub.find_named(name, except_id=self_id)
        return target, body, err, name
    peers = hub.named_except(self_id)
    rest_l = rest.lower()
    best = None
    best_len = -1
    for peer in peers:
        name = (peer.name or "").strip()
        if not name:
            continue
        low = name.lower()
        if rest_l == low or rest_l.startswith(low + " "):
            if len(name) > best_len:
                best = peer
                best_len = len(name)
    if best:
        return best, rest[best_len:].strip(), "", best.name
    token, _, leftover = rest.partition(" ")
    target, err = hub.find_named(token, except_id=self_id)
    if target:
        return target, leftover.strip(), "", target.name
    return None, leftover.strip(), err or "missing", token


def _deliver_whisper(hub: "Hub", sender: "WSClient", data: dict, text: str) -> None:
    to_id = str(data.get("to") or "").strip()
    is_cmd, rest = _split_whisper(text)
    target = None
    err = ""
    hint = ""
    body = text
    if to_id:
        other = hub.get(to_id)
        if other is sender or other is not None and other.peer_id == sender.peer_id:
            err = "self"
            hint = sender.name
        elif other is None or not other.named:
            err = "missing"
            hint = to_id
        else:
            target = other
            body = rest if is_cmd else text
            if is_cmd:
                body = _strip_name_prefix(body, other.name)
    if target is None and not err:
        if is_cmd:
            target, body, err, hint = _resolve_whisper_rest(rest, hub, sender.peer_id)
        else:
            err = "missing"
            hint = to_id or ""
    if err or target is None:
        hub.send_json(
            sender,
            {
                "type": "chat",
                "sys": True,
                "text": _whisper_error(err or "missing", hint),
                "ts": int(time.time() * 1000),
            },
        )
        return
    body = str(body or "").strip()[:400]
    if not body:
        hub.send_json(
            sender,
            {
                "type": "chat",
                "sys": True,
                "text": _whisper_error("empty"),
                "ts": int(time.time() * 1000),
            },
        )
        return
    payload = {
        "type": "chat",
        "id": sender.peer_id,
        "name": sender.name,
        "text": body,
        "ts": int(time.time() * 1000),
        "whisper": True,
        "to": target.peer_id,
        "toName": target.name,
    }
    raw = json.dumps(payload, separators=(",", ":"))
    sender.send_text(raw)
    if target is not sender:
        target.send_text(raw)


class Hub:
    def __init__(self) -> None:
        self.lock = threading.Lock()
        self.clients: dict[str, "WSClient"] = {}
        self.last_mix = None
        self.last_map = None
        self.last_chars: dict[str, dict] = {}
        self.last_rolls: list[dict] = []
        self.last_pics: dict[str, str] = {}

    def add(self, client: "WSClient") -> None:
        with self.lock:
            self.clients[client.peer_id] = client
        self.broadcast_peers()

    def remove(self, client: "WSClient") -> None:
        with self.lock:
            current = self.clients.get(client.peer_id)
            if current is client:
                self.clients.pop(client.peer_id, None)
                self.last_chars.pop(client.peer_id, None)
                self.last_pics.pop(client.peer_id, None)
                if client.role == "host":
                    self.last_mix = None
                    self.last_map = None
        gone = client.peer_id
        self.broadcast_peers()
        self.broadcast({"type": "chars", "from": gone, "name": "", "list": []})

    def peers(self) -> list[dict]:
        with self.lock:
            return [
                {"id": c.peer_id, "name": c.name, "role": c.role}
                for c in self.clients.values()
                if c.named
            ]

    def get(self, peer_id: str) -> "WSClient | None":
        with self.lock:
            return self.clients.get(peer_id)

    def named_except(self, except_id: str | None = None) -> list["WSClient"]:
        with self.lock:
            return [
                c
                for c in self.clients.values()
                if c.named and c.peer_id != except_id
            ]

    def find_named(self, needle: str, except_id: str | None = None) -> tuple["WSClient | None", str]:
        want = _fold_name(needle)
        if not want:
            return None, "missing"
        named = self.named_except(except_id)
        exact = [c for c in named if _fold_name(c.name) == want]
        if len(exact) == 1:
            return exact[0], ""
        if len(exact) > 1:
            return None, "ambiguous"
        prefixes = [c for c in named if _fold_name(c.name).startswith(want)]
        if len(prefixes) == 1:
            return prefixes[0], ""
        if len(prefixes) > 1:
            return None, "ambiguous"
        return None, "missing"

    def send_json(self, client: "WSClient", msg: dict) -> None:
        client.send_text(json.dumps(msg, separators=(",", ":")))

    def remember_roll(self, row: dict) -> None:
        with self.lock:
            self.last_rolls.append(row)
            if len(self.last_rolls) > 40:
                self.last_rolls = self.last_rolls[-40:]

    def rolls(self) -> list[dict]:
        with self.lock:
            return list(self.last_rolls)

    def pics_table(self) -> list[dict]:
        with self.lock:
            return [{"from": pid, "pic": pic} for pid, pic in self.last_pics.items() if pic]

    def broadcast(self, msg: dict, exclude: str | None = None) -> None:
        payload = json.dumps(msg, separators=(",", ":"))
        with self.lock:
            targets = list(self.clients.values())
        for client in targets:
            if exclude and client.peer_id == exclude:
                continue
            client.send_text(payload)

    def broadcast_peers(self) -> None:
        self.broadcast({"type": "peers", "peers": self.peers()})

    def chars_table(self) -> list[dict]:
        with self.lock:
            out = []
            for pid, pack in self.last_chars.items():
                out.append({"from": pid, "name": pack.get("name") or "", "list": pack.get("list") or []})
            return out

    def host(self) -> "WSClient | None":
        with self.lock:
            for c in self.clients.values():
                if c.role == "host" and c.named:
                    return c
        return None


GITHUB_REPO = "Steelworth/Blightnet"
GITHUB_BRANCH = "main"
GITHUB_API = "https://api.github.com"
UPDATE_SHA_PATH = os.path.join(ROOT, ".blightnet-sha")
UPDATE_UA = "Blightnet-Updater"
UPDATE_SKIP_PREFIX = (
    "uploads/",
    ".git/",
    "tools/_go/",
    "tools/_raw/",
    "tools/_wav/",
    "__pycache__/",
)
UPDATE_SERVER_FILES = {
    "serve.py",
    "blightnet_window.py",
    "browser.py",
    "start.sh",
    "start.bat",
    "Blightnet.exe",
    "Hearthsong.exe",
}
_update_lock = threading.Lock()
_update_job: dict = {
    "running": False,
    "phase": "idle",
    "message": "",
    "checked": 0,
    "changed": 0,
    "files": [],
    "sha": "",
    "reload": False,
    "restart": False,
    "error": None,
}


def _update_snapshot() -> dict:
    with _update_lock:
        return dict(_update_job)


def _update_set(**kwargs) -> None:
    with _update_lock:
        _update_job.update(kwargs)


def git_blob_sha(data: bytes) -> str:
    return hashlib.sha1(b"blob %d\0" % len(data) + data).hexdigest()


def update_safe_rel(rel: str) -> str | None:
    rel = str(rel or "").replace("\\", "/").lstrip("/")
    if not rel or rel in (".", "..") or rel.startswith("../") or "/../" in rel or "\x00" in rel:
        return None
    lower = rel.lower()
    for prefix in UPDATE_SKIP_PREFIX:
        if lower == prefix.rstrip("/") or lower.startswith(prefix):
            return None
    parts = rel.split("/")
    if any(p == "__pycache__" or p.endswith(".pyc") for p in parts):
        return None
    if parts[-1] in (".blightnet-sha",) or parts[-1].endswith(".blightnet-new"):
        return None
    return rel


def _http_get(url: str, timeout: int = 45) -> bytes:
    req = urllib.request.Request(
        url,
        headers={
            "User-Agent": UPDATE_UA,
            "Accept": "application/vnd.github+json, application/octet-stream, */*",
        },
    )
    with urllib.request.urlopen(req, timeout=timeout) as resp:
        final = getattr(resp, "geturl", lambda: url)()
        host = urllib.parse.urlparse(final).netloc.lower()
        if host not in (
            "api.github.com",
            "raw.githubusercontent.com",
            "codeload.github.com",
            "github.com",
            "objects.githubusercontent.com",
        ):
            raise OSError("unexpected host " + host)
        return resp.read()


def _http_json(url: str):
    raw = _http_get(url)
    return json.loads(raw.decode("utf-8"))


def _read_saved_sha() -> str:
    try:
        return open(UPDATE_SHA_PATH, encoding="utf-8").read().strip()[:40]
    except OSError:
        return ""


def _write_saved_sha(sha: str) -> None:
    try:
        with open(UPDATE_SHA_PATH, "w", encoding="utf-8") as f:
            f.write(sha + "\n")
    except OSError:
        pass


def _git_bin() -> str | None:
    return shutil.which("git")


def _git(args: list[str], timeout: int = 120) -> subprocess.CompletedProcess | None:
    git = _git_bin()
    if not git or not os.path.isdir(os.path.join(ROOT, ".git")):
        return None
    env = os.environ.copy()
    env["GIT_TERMINAL_PROMPT"] = "0"
    env["GIT_SSH_COMMAND"] = "ssh -o BatchMode=yes -o StrictHostKeyChecking=accept-new"
    try:
        return subprocess.run(
            [git, *args],
            cwd=ROOT,
            capture_output=True,
            text=True,
            timeout=timeout,
            env=env,
        )
    except Exception:
        return None


def _github_head_sha() -> str:
    data = _http_json(f"{GITHUB_API}/repos/{GITHUB_REPO}/commits/{GITHUB_BRANCH}")
    sha = str(data.get("sha") or "")
    if len(sha) < 7:
        raise OSError("GitHub did not return a commit")
    return sha


def _write_rel(rel: str, data: bytes) -> None:
    dest = os.path.join(ROOT, *rel.split("/"))
    parent = os.path.dirname(dest)
    if parent:
        os.makedirs(parent, exist_ok=True)
    base = os.path.basename(rel)
    if sys.platform == "win32" and base in ("Blightnet.exe", "Hearthsong.exe"):
        with open(dest + ".new", "wb") as f:
            f.write(data)
        return
    tmp = dest + ".blightnet-new"
    with open(tmp, "wb") as f:
        f.write(data)
    try:
        os.replace(tmp, dest)
    except OSError:
        try:
            os.remove(tmp)
        except OSError:
            pass
        raise


def _try_git_update() -> bool:
    inside = _git(["rev-parse", "--is-inside-work-tree"])
    if not inside or inside.returncode != 0:
        return False
    _update_set(phase="checking", message="Checking GitHub…")
    old = _git(["rev-parse", "HEAD"])
    old_sha = (old.stdout or "").strip() if old and old.returncode == 0 else ""
    fetch = _git(["fetch", "--quiet", f"https://github.com/{GITHUB_REPO}.git", GITHUB_BRANCH], timeout=180)
    if not fetch or fetch.returncode != 0:
        fetch = _git(["fetch", "--quiet", "origin", GITHUB_BRANCH], timeout=180)
    if not fetch or fetch.returncode != 0:
        return False
    new = _git(["rev-parse", "FETCH_HEAD"])
    new_sha = (new.stdout or "").strip() if new and new.returncode == 0 else ""
    if not new_sha:
        return False
    if old_sha and new_sha == old_sha:
        _update_set(phase="done", message="Up to date", sha=new_sha, changed=0, files=[], reload=False, restart=False)
        _write_saved_sha(new_sha)
        return True
    names = []
    if old_sha:
        diff = _git(["diff", "--name-only", old_sha, new_sha], timeout=60)
        if diff and diff.returncode == 0:
            names = [ln.strip() for ln in (diff.stdout or "").splitlines() if ln.strip()]
    _update_set(message="Downloading updates…", phase="downloading")
    merge = _git(["merge", "--ff-only", new_sha], timeout=180)
    if not merge or merge.returncode != 0:
        return False
    safe = [n for n in names if update_safe_rel(n)]
    restart = any(n in UPDATE_SERVER_FILES or n.startswith("gst/") for n in safe)
    pulled = new_sha != old_sha
    n = len(safe) if safe else int(pulled)
    _update_set(
        phase="done",
        message=("Updated " + str(n) + " file" + ("s" if n != 1 else "")) if n else "Up to date",
        sha=new_sha,
        changed=n,
        files=safe[:80],
        reload=pulled and not restart,
        restart=restart,
    )
    _write_saved_sha(new_sha)
    return True


def _try_api_update() -> None:
    _update_set(phase="checking", message="Checking GitHub…")
    head = _github_head_sha()
    saved = _read_saved_sha()
    local_head = ""
    rev = _git(["rev-parse", "HEAD"])
    if rev and rev.returncode == 0:
        local_head = (rev.stdout or "").strip()
    current = local_head or saved
    if current and current == head:
        _update_set(phase="done", message="Up to date", sha=head, changed=0, files=[], reload=False, restart=False)
        _write_saved_sha(head)
        return
    tree = _http_json(f"{GITHUB_API}/repos/{GITHUB_REPO}/git/trees/{head}?recursive=1")
    blobs = [e for e in (tree.get("tree") or []) if e.get("type") == "blob" and update_safe_rel(e.get("path") or "")]
    changed: list[str] = []
    restart = False
    total = len(blobs)
    for i, entry in enumerate(blobs, 1):
        rel = update_safe_rel(entry.get("path") or "")
        if not rel:
            continue
        if i % 25 == 0 or i == total:
            _update_set(checked=i, message=f"Checking {i}/{total}…")
        want = str(entry.get("sha") or "")
        dest = os.path.join(ROOT, *rel.split("/"))
        have = ""
        try:
            with open(dest, "rb") as f:
                have = git_blob_sha(f.read())
        except OSError:
            have = ""
        if have and want and have == want:
            continue
        _update_set(phase="downloading", message=f"Downloading {rel}…", checked=i)
        url = f"https://raw.githubusercontent.com/{GITHUB_REPO}/{head}/{urllib.parse.quote(rel)}"
        data = _http_get(url, timeout=90)
        _write_rel(rel, data)
        changed.append(rel)
        if rel in UPDATE_SERVER_FILES or rel.startswith("gst/"):
            restart = True
        _update_set(changed=len(changed), files=changed[:80])
    _write_saved_sha(head)
    _update_set(
        phase="done",
        message=("Updated " + str(len(changed)) + " file" + ("s" if len(changed) != 1 else "")) if changed else "Up to date",
        sha=head,
        changed=len(changed),
        files=changed[:80],
        reload=bool(changed) and not restart,
        restart=restart,
    )


def _run_update_job() -> None:
    try:
        if _try_git_update():
            return
        _try_api_update()
    except urllib.error.HTTPError as exc:
        msg = "GitHub returned " + str(exc.code)
        if exc.code == 403:
            msg = "GitHub rate limit. Try again in a few minutes."
        _update_set(phase="error", error=msg, message=msg)
    except Exception as exc:
        msg = str(exc)[:160] or "Update failed"
        _update_set(phase="error", error=msg, message=msg)
    finally:
        with _update_lock:
            _update_job["running"] = False
            if not _update_job.get("phase") or _update_job["phase"] in ("checking", "downloading", "applying"):
                _update_job["phase"] = "error"
                _update_job["error"] = _update_job.get("error") or "Update stopped"
                _update_job["message"] = _update_job["error"]


def start_update_job() -> dict:
    with _update_lock:
        if _update_job.get("running"):
            return dict(_update_job)
        _update_job.update(
            {
                "running": True,
                "phase": "checking",
                "message": "Checking GitHub…",
                "checked": 0,
                "changed": 0,
                "files": [],
                "sha": "",
                "reload": False,
                "restart": False,
                "error": None,
            }
        )
    threading.Thread(target=_run_update_job, daemon=True, name="blightnet-update").start()
    return _update_snapshot()


class WSClient:
    def __init__(self, handler: "Handler") -> None:
        self.handler = handler
        self.peer_id = uuid.uuid4().hex[:12]
        self.name = ""
        self.role = "guest"
        self.named = False
        self.pic = ""
        self.lock = threading.Lock()
        self.alive = True

    def send_text(self, text: str) -> None:
        if not self.alive:
            return
        data = text.encode("utf-8")
        header = bytearray([0x81])
        n = len(data)
        if n < 126:
            header.append(n)
        elif n < 65536:
            header.append(126)
            header.extend(struct.pack("!H", n))
        else:
            header.append(127)
            header.extend(struct.pack("!Q", n))
        try:
            with self.lock:
                self.handler.wfile.write(header + data)
                self.handler.wfile.flush()
        except Exception:
            self.alive = False


class Handler(http.server.SimpleHTTPRequestHandler):
    protocol_version = "HTTP/1.1"
    extensions_map = {
        **http.server.SimpleHTTPRequestHandler.extensions_map,
        ".ogg": "audio/ogg",
        ".oga": "audio/ogg",
        ".js": "text/javascript",
        ".mjs": "text/javascript",
        ".css": "text/css",
        ".json": "application/json",
        ".png": "image/png",
        ".jpg": "image/jpeg",
        ".jpeg": "image/jpeg",
        ".svg": "image/svg+xml",
        ".wav": "audio/wav",
        ".mp3": "audio/mpeg",
        ".m4a": "audio/mp4",
        ".flac": "audio/flac",
        ".mp4": "video/mp4",
        ".webm": "video/webm",
    }

    def send_response(self, code, message=None):
        self._status = code
        super().send_response(code, message)

    def end_headers(self):
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Access-Control-Allow-Headers", "Content-Type, X-Hearth-Name, X-Hearth-Category, X-Hearth-Id, X-Hearth-Icon, X-Hearth-Mood")
        self.send_header("Access-Control-Allow-Methods", "GET, POST, OPTIONS, HEAD")
        path = self.path.split("?", 1)[0]
        code = getattr(self, "_status", 200)
        if code >= 400:
            self.send_header("Cache-Control", "no-store")
        elif path in ("/", "/index.html") or path.endswith((".html", ".js", ".mjs", ".css")) or path.startswith("/data/"):
            self.send_header("Cache-Control", "no-store")
        elif path.startswith("/assets/") or path.startswith("/audio/") or path.startswith("/uploads/"):
            self.send_header("Cache-Control", "public, max-age=86400")
            self.send_header("Accept-Ranges", "bytes")
        super().end_headers()

    def do_OPTIONS(self):
        self.send_response(204)
        self.end_headers()

    def copyfile(self, source, outputfile):
        try:
            super().copyfile(source, outputfile)
        except (BrokenPipeError, ConnectionResetError, ConnectionAbortedError):
            return

    def list_directory(self, path):
        self.send_error(404, "Not found")

    def log_message(self, fmt, *args):
        sys.stderr.write("%s - %s\n" % (self.address_string(), fmt % args))

    def log_error(self, fmt, *args):
        msg = fmt % args
        if "Broken pipe" in msg or "Connection reset" in msg:
            return
        super().log_error(fmt, *args)

    def _json(self, code: int, obj) -> None:
        body = json.dumps(obj).encode("utf-8")
        self.send_response(code)
        self.send_header("Content-Type", "application/json")
        self.send_header("Cache-Control", "no-store")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        if self.headers.get("Upgrade", "").lower() == "websocket":
            self._ws_serve()
            return
        path = self.path.split("?", 1)[0]
        if path in ("/__hearthsong", "/__hearthsong/", "/__blightnet", "/__blightnet/", "/api/info"):
            port = self.server.server_address[1]
            self._json(
                200,
                {
                    "ok": True,
                    "app": "blightnet",
                    "port": port,
                    "ips": lan_ips(),
                    "ip6": ipv6_ips(),
                    "wan": NET.get("wan") or "",
                    "wan6": NET.get("wan6") or "",
                    "upnp": bool(NET.get("upnp")),
                    "relay": NET.get("relay") or "",
                    "url": f"http://127.0.0.1:{port}/",
                },
            )
            return
        if path in ("/api/uploads", "/api/uploads/"):
            self._json(200, {"files": load_manifest()})
            return
        if path in ("/api/quit", "/api/quit/"):
            self._json(200, {"ok": True})
            request_shutdown()
            return
        if path in ("/api/displays", "/api/displays/"):
            self._json(200, {"ok": True, "displays": list_displays()})
            return
        if path in ("/api/update", "/api/update/"):
            snap = _update_snapshot()
            snap["ok"] = not snap.get("error")
            self._json(200, snap)
            return
        super().do_GET()

    def do_POST(self):
        path = self.path.split("?", 1)[0]
        if path in ("/api/upload", "/api/upload/"):
            self._handle_upload()
            return
        if path in ("/api/quit", "/api/quit/"):
            self._json(200, {"ok": True})
            request_shutdown()
            return
        if path in ("/api/update", "/api/update/"):
            snap = start_update_job()
            snap["ok"] = True
            self._json(200, snap)
            return
        if path in ("/api/display", "/api/display/"):
            length = int(self.headers.get("Content-Length", "0") or "0")
            raw = self.rfile.read(length) if length > 0 and length < 4096 else b"{}"
            try:
                data = json.loads(raw.decode("utf-8"))
            except Exception:
                data = {}
            name = str((data or {}).get("name") or "")
            rec = place_blightnet_window(name) if name else None
            if not rec:
                self._json(400, {"ok": False, "error": "unknown display"})
                return
            self._json(200, {"ok": True, "display": rec, "displays": list_displays()})
            return
        self.send_error(404, "Not found")

    def _handle_upload(self) -> None:
        length = int(self.headers.get("Content-Length", "0") or "0")
        if length <= 0 or length > MAX_UPLOAD:
            self._json(400, {"ok": False, "error": "bad size"})
            return
        query = urllib.parse.parse_qs(urllib.parse.urlparse(self.path).query)
        name = (query.get("name") or [self.headers.get("X-Hearth-Name") or "Upload"])[0][:80]
        category = (query.get("category") or [self.headers.get("X-Hearth-Category") or "music"])[0]
        if category not in ("music", "weather", "animals", "ambience", "map"):
            category = "music"
        file_id = (query.get("id") or [self.headers.get("X-Hearth-Id") or ""])[0]
        file_id = re.sub(r"[^a-zA-Z0-9_-]", "", file_id)[:40] or ("up-" + uuid.uuid4().hex[:10])
        icon = (query.get("icon") or [self.headers.get("X-Hearth-Icon") or "spark"])[0][:24]
        mood = (query.get("mood") or [self.headers.get("X-Hearth-Mood") or "calm"])[0][:24]
        mime = self.headers.get("Content-Type") or "application/octet-stream"
        ext = {
            "audio/ogg": ".ogg",
            "audio/mpeg": ".mp3",
            "audio/wav": ".wav",
            "audio/x-wav": ".wav",
            "audio/mp4": ".m4a",
            "audio/flac": ".flac",
            "audio/webm": ".webm",
            "image/jpeg": ".jpg",
            "image/jpg": ".jpg",
            "image/png": ".png",
            "image/webp": ".webp",
            "image/gif": ".gif",
        }.get(mime.split(";")[0].strip(), "")
        if not ext:
            guess = os.path.splitext(name)[1].lower()
            if guess in AUDIO_EXT or guess in IMAGE_EXT:
                ext = ".jpg" if guess == ".jpeg" else guess
            else:
                ext = ".jpg" if category == "map" else ".ogg"
        raw = self.rfile.read(length)
        os.makedirs(UPLOAD_DIR, exist_ok=True)
        filename = file_id + ext
        dest = os.path.join(UPLOAD_DIR, filename)
        with open(dest, "wb") as f:
            f.write(raw)
        rec = {
            "id": file_id,
            "name": name,
            "category": category,
            "mime": mime.split(";")[0].strip(),
            "size": len(raw),
            "icon": icon,
            "mood": mood,
            "file": "uploads/" + filename,
        }
        rows = [r for r in load_manifest() if r.get("id") != file_id]
        rows.append(rec)
        save_manifest(rows)
        self._json(200, {"ok": True, "file": rec})

    def _ws_serve(self) -> None:
        key = self.headers.get("Sec-WebSocket-Key")
        if not key:
            self.send_error(400, "Missing key")
            return
        accept = base64.b64encode(hashlib.sha1((key + WS_GUID).encode("utf-8")).digest()).decode("ascii")
        self.send_response(101, "Switching Protocols")
        self.send_header("Upgrade", "websocket")
        self.send_header("Connection", "Upgrade")
        self.send_header("Sec-WebSocket-Accept", accept)
        self.end_headers()
        client = WSClient(self)
        self.ws_client = client
        hub: Hub = self.server.hub
        try:
            self._ws_loop(client, hub)
        finally:
            hub.remove(client)

    def _ws_loop(self, client: WSClient, hub: Hub) -> None:
        while client.alive:
            msg = self._ws_read()
            if msg is None:
                break
            if msg == "":
                continue
            try:
                data = json.loads(msg)
            except Exception:
                continue
            if not isinstance(data, dict):
                continue
            kind = data.get("type")
            if kind == "hello":
                name = str(data.get("name") or "Traveller").strip()[:24] or "Traveller"
                role = data.get("role") if data.get("role") in ("host", "guest") else "guest"
                want_id = re.sub(r"[^a-zA-Z0-9_-]", "", str(data.get("id") or ""))[:24]
                with hub.lock:
                    if want_id:
                        old = hub.clients.get(want_id)
                        if old is None or old is client:
                            client.peer_id = want_id
                        else:
                            old.alive = False
                            hub.clients.pop(want_id, None)
                            client.peer_id = want_id
                    if role == "host":
                        for other in hub.clients.values():
                            if other is not client and other.role == "host":
                                role = "guest"
                                break
                    client.name = name
                    client.role = role
                    client.named = True
                    pic = _clean_pic(data.get("pic"))
                    client.pic = pic
                    hub.last_pics[client.peer_id] = pic
                    hub.clients[client.peer_id] = client
                client.send_text(
                    json.dumps(
                        {
                            "type": "welcome",
                            "id": client.peer_id,
                            "role": client.role,
                            "peers": hub.peers(),
                            "files": load_manifest(),
                            "mix": hub.last_mix,
                            "map": hub.last_map,
                            "chars": hub.chars_table(),
                            "rolls": hub.rolls(),
                            "pics": hub.pics_table(),
                        }
                    )
                )
                hub.broadcast_peers()
                if client.pic:
                    hub.broadcast(
                        {
                            "type": "profile",
                            "from": client.peer_id,
                            "name": client.name,
                            "pic": client.pic,
                        },
                        exclude=client.peer_id,
                    )
                continue
            if not client.named:
                continue
            if kind == "chat":
                text = str(data.get("text") or "").strip()[:400]
                if not text:
                    continue
                to_id = str(data.get("to") or "").strip()
                is_cmd, _ = _split_whisper(text)
                if to_id or is_cmd or data.get("whisper"):
                    _deliver_whisper(hub, client, data, text)
                    continue
                hub.broadcast(
                    {
                        "type": "chat",
                        "id": client.peer_id,
                        "name": client.name,
                        "text": text,
                        "ts": int(time.time() * 1000),
                    }
                )
            elif kind == "mix":
                if client.role != "host":
                    continue
                mix = data.get("mix")
                if isinstance(mix, dict):
                    mix = dict(mix)
                    mix.pop("master", None)
                    hub.last_mix = mix
                    hub.broadcast({"type": "mix", "mix": mix, "from": client.peer_id}, exclude=client.peer_id)
            elif kind == "roll":
                who = str(data.get("who") or client.name or "Character").strip()[:48] or "Character"
                action = str(data.get("action") or "roll").strip()[:80] or "roll"
                try:
                    pct = int(data.get("pct"))
                except (TypeError, ValueError):
                    continue
                pct = max(0, min(100, pct))
                grade = str(data.get("grade") or "").strip()[:24]
                dmg = data.get("damage")
                if dmg is not None and dmg != "":
                    try:
                        dmg = int(dmg)
                    except (TypeError, ValueError):
                        dmg = None
                else:
                    dmg = None
                player = str(data.get("player") or client.name or "").strip()[:24]
                adv = str(data.get("adv") or "").strip()[:8]
                if adv not in ("adv", "dis"):
                    adv = ""

                def _pct_opt(key):
                    raw = data.get(key)
                    if raw is None or raw == "":
                        return None
                    try:
                        n = int(raw)
                    except (TypeError, ValueError):
                        return None
                    return max(0, min(100, n))

                pct_a = _pct_opt("pctA")
                pct_b = _pct_opt("pctB")
                row = {
                    "type": "roll",
                    "from": client.peer_id,
                    "player": player,
                    "who": who,
                    "action": action,
                    "pct": pct,
                    "grade": grade,
                    "damage": dmg,
                    "taken": bool(data.get("taken")),
                    "heal": bool(data.get("heal")),
                    "hack": bool(data.get("hack")),
                    "deathSave": bool(data.get("deathSave")),
                    "targetId": str(data.get("targetId") or "")[:64],
                    "targetName": str(data.get("targetName") or "").strip()[:48],
                    "targetOwner": str(data.get("targetOwner") or "")[:24],
                    "tokenId": str(data.get("tokenId") or "")[:32],
                    "sheetId": str(data.get("sheetId") or "")[:64],
                    "sheetOwner": str(data.get("sheetOwner") or "")[:24],
                    "adv": adv,
                    "pctA": pct_a,
                    "pctB": pct_b,
                    "ts": int(time.time() * 1000),
                }
                hub.remember_roll(row)
                hub.broadcast(row, exclude=client.peer_id)
            elif kind == "log":
                who = str(data.get("who") or client.name or "Character").strip()[:48] or "Character"
                action = str(data.get("action") or "look").strip()[:80] or "look"
                text = str(data.get("text") or "")[:800]
                player = str(data.get("player") or client.name or "").strip()[:24]
                row = {
                    "type": "log",
                    "info": True,
                    "from": client.peer_id,
                    "player": player,
                    "who": who,
                    "action": action,
                    "text": text,
                    "ts": int(time.time() * 1000),
                }
                hub.remember_roll(row)
                hub.broadcast(row, exclude=client.peer_id)
            elif kind == "map":
                if client.role != "host":
                    continue
                hub.last_map = data.get("map")
                hub.broadcast({"type": "map", "map": hub.last_map, "from": client.peer_id}, exclude=client.peer_id)
            elif kind == "map-edit":
                host = hub.host()
                if host and host.peer_id != client.peer_id:
                    host.send_text(
                        json.dumps(
                            {
                                "type": "map-edit",
                                "from": client.peer_id,
                                "edit": data.get("edit"),
                            }
                        )
                    )
            elif kind == "signal":
                target = str(data.get("to") or "")
                payload = data.get("payload")
                other = hub.get(target)
                if other:
                    other.send_text(
                        json.dumps(
                            {
                                "type": "signal",
                                "from": client.peer_id,
                                "to": target,
                                "payload": payload,
                            }
                        )
                    )
            elif kind == "voice":
                action = str(data.get("action") or "")
                if action not in ("invite", "accept", "decline", "hangup"):
                    continue
                target = str(data.get("to") or "").strip()
                other = hub.get(target)
                if other and other.named:
                    other.send_text(
                        json.dumps(
                            {
                                "type": "voice",
                                "action": action,
                                "from": client.peer_id,
                                "name": client.name,
                                "to": target,
                            }
                        )
                    )
            elif kind == "profile":
                pic = _clean_pic(data.get("pic"))
                client.pic = pic
                with hub.lock:
                    if pic:
                        hub.last_pics[client.peer_id] = pic
                    else:
                        hub.last_pics.pop(client.peer_id, None)
                hub.broadcast(
                    {
                        "type": "profile",
                        "from": client.peer_id,
                        "name": client.name,
                        "pic": pic,
                    },
                    exclude=client.peer_id,
                )
            elif kind == "chars":
                rows = data.get("list")
                if not isinstance(rows, list):
                    rows = []
                rows = rows[:16]
                with hub.lock:
                    hub.last_chars[client.peer_id] = {"name": client.name, "list": rows}
                hub.broadcast(
                    {"type": "chars", "from": client.peer_id, "name": client.name, "list": rows},
                    exclude=client.peer_id,
                )
            elif kind == "files":
                if client.role == "host":
                    hub.broadcast({"type": "files", "files": data.get("files") or []}, exclude=client.peer_id)
            elif kind == "ping":
                client.send_text(json.dumps({"type": "pong"}))

    def _ws_read(self) -> str | None:
        try:
            header = self.rfile.read(2)
            if not header or len(header) < 2:
                return None
            b1, b2 = header[0], header[1]
            opcode = b1 & 0x0F
            masked = (b2 & 0x80) != 0
            length = b2 & 0x7F
            if length == 126:
                ext = self.rfile.read(2)
                if len(ext) < 2:
                    return None
                length = struct.unpack("!H", ext)[0]
            elif length == 127:
                ext = self.rfile.read(8)
                if len(ext) < 8:
                    return None
                length = struct.unpack("!Q", ext)[0]
            if length > MAX_WS:
                return None
            mask = self.rfile.read(4) if masked else b""
            data = self.rfile.read(length) if length else b""
            if len(data) < length:
                return None
            if masked:
                data = bytes(b ^ mask[i % 4] for i, b in enumerate(data))
            if opcode == 0x8:
                return None
            if opcode == 0x9:
                self._ws_pong(data)
                return ""
            if opcode in (0x1, 0x2):
                return data.decode("utf-8", "replace")
            return ""
        except Exception:
            return None

    def _ws_pong(self, data: bytes) -> None:
        header = bytearray([0x8A])
        n = len(data)
        if n < 126:
            header.append(n)
        elif n < 65536:
            header.append(126)
            header.extend(struct.pack("!H", n))
        else:
            header.append(127)
            header.extend(struct.pack("!Q", n))
        lock = getattr(getattr(self, "ws_client", None), "lock", None)
        try:
            if lock:
                with lock:
                    self.wfile.write(header + data)
                    self.wfile.flush()
            else:
                self.wfile.write(header + data)
                self.wfile.flush()
        except Exception:
            pass


class Server(http.server.ThreadingHTTPServer):
    allow_reuse_address = True
    daemon_threads = True
    request_queue_size = 256
    address_family = socket.AF_INET6

    def server_bind(self):
        try:
            self.socket.setsockopt(socket.IPPROTO_IPV6, socket.IPV6_V6ONLY, 0)
        except Exception:
            pass
        super().server_bind()

    def __init__(self, addr, handler):
        super().__init__(addr, handler)
        self.hub = Hub()


def port_free(port: int) -> bool:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        try:
            s.bind(("0.0.0.0", port))
            return True
        except OSError:
            return False


def pick_port(preferred: int) -> int:
    if port_free(preferred):
        return preferred
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.bind(("0.0.0.0", 0))
        return s.getsockname()[1]


def listener_pids(port: int) -> list[int]:
    try:
        out = subprocess.check_output(
            ["ss", "-H", "-tlnp", f"( sport = :{port} )"],
            text=True,
            stderr=subprocess.DEVNULL,
        )
    except Exception:
        return []
    found = []
    for pid in re.findall(r"pid=(\d+)", out):
        n = int(pid)
        if n not in found:
            found.append(n)
    return found


def cmdline_of(pid: int) -> str:
    try:
        raw = open(f"/proc/{pid}/cmdline", "rb").read().replace(b"\0", b" ")
        return raw.decode("utf-8", "ignore")
    except Exception:
        return ""


def reclaim_port(port: int) -> None:
    """Stop an earlier Blightnet server so ./start.sh always lands on 8765."""
    for pid in listener_pids(port):
        if pid == os.getpid():
            continue
        cmd = cmdline_of(pid)
        if "serve.py" not in cmd:
            continue
        try:
            os.kill(pid, signal.SIGTERM)
        except OSError:
            continue
        for _ in range(20):
            if port_free(port):
                return
            time.sleep(0.05)
        try:
            os.kill(pid, signal.SIGKILL)
        except OSError:
            pass
        time.sleep(0.1)


def list_displays() -> list[dict]:
    rows: list[dict] = []
    try:
        out = subprocess.check_output(["xrandr", "--current"], text=True, stderr=subprocess.DEVNULL)
    except Exception:
        out = ""
    current = None
    for line in out.splitlines():
        m = re.match(
            r"^(\S+)\s+connected(?:\s+primary)?\s+(\d+)x(\d+)\+(\d+)\+(\d+)",
            line,
        )
        if m:
            current = {
                "name": m.group(1),
                "w": int(m.group(2)),
                "h": int(m.group(3)),
                "x": int(m.group(4)),
                "y": int(m.group(5)),
                "hz": 0,
                "primary": " primary " in f" {line} ",
            }
            rows.append(current)
            continue
        if current and "*" in line:
            hm = re.search(r"(\d+(?:\.\d+)?)\*", line)
            if hm:
                try:
                    current["hz"] = round(float(hm.group(1)), 1)
                except Exception:
                    pass
    pref = load_display_pref()
    for rec in rows:
        rec["on"] = rec["name"] == pref if pref else rec.get("primary")
    if rows and not any(r["on"] for r in rows):
        rows[0]["on"] = True
    return rows


def load_display_pref() -> str:
    try:
        with open(DISPLAY_PREF, encoding="utf-8") as f:
            data = json.load(f)
        name = str(data.get("name") or "")
        if name:
            return name
    except Exception:
        pass
    return ""


def save_display_pref(name: str) -> None:
    os.makedirs(os.path.dirname(DISPLAY_PREF), exist_ok=True)
    tmp = DISPLAY_PREF + ".tmp"
    with open(tmp, "w", encoding="utf-8") as f:
        json.dump({"name": name}, f)
    os.replace(tmp, DISPLAY_PREF)


def _xdotool(*args: str) -> str:
    return subprocess.check_output(["xdotool", *args], text=True, stderr=subprocess.DEVNULL).strip()


def _window_area(wid: str) -> int:
    try:
        geo = _xdotool("getwindowgeometry", wid)
    except Exception:
        return 0
    m = re.search(r"Geometry:\s+(\d+)x(\d+)", geo)
    if not m:
        return 0
    return int(m.group(1)) * int(m.group(2))


def _blightnet_window_ids() -> list[str]:
    found: list[str] = []

    def add(wid: str) -> None:
        if wid and wid not in found and wid.isdigit():
            found.append(wid)

    pids: list[int] = []
    proc = _window_proc
    if proc is not None and proc.poll() is None:
        pids.append(int(proc.pid))
        try:
            kids = subprocess.check_output(["pgrep", "-P", str(proc.pid)], text=True, stderr=subprocess.DEVNULL)
            pids.extend(int(x) for x in kids.split() if x.isdigit())
        except Exception:
            pass
    for pid in pids:
        try:
            for wid in _xdotool("search", "--pid", str(pid)).split():
                add(wid)
        except Exception:
            pass
    for key in ("--class", "--name"):
        try:
            for name in ("Blightnet", "Blighnet"):
                for wid in _xdotool("search", key, name).split():
                    add(wid)
        except Exception:
            pass
    found.sort(key=_window_area, reverse=True)
    return found


def place_blightnet_window(name: str) -> dict | None:
    displays = list_displays()
    rec = next((d for d in displays if d["name"] == name), None)
    if not rec:
        return None
    save_display_pref(name)
    x, y, w, h = rec["x"], rec["y"], rec["w"], rec["h"]
    ids = _blightnet_window_ids()
    if not ids:
        print(f"Blightnet window: none found to move to {name}", flush=True)
        return rec
    wid = ids[0]
    try:
        subprocess.call(
            ["xdotool", "windowstate", "--remove", "FULLSCREEN", wid],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        subprocess.call(
            ["xdotool", "windowstate", "--remove", "MAXIMIZED_VERT", wid],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        subprocess.call(
            ["xdotool", "windowstate", "--remove", "MAXIMIZED_HORZ", wid],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        time.sleep(0.18)
        subprocess.check_call(
            ["xdotool", "windowmove", wid, str(x), str(y), "windowsize", wid, str(max(800, w - 1)), str(max(600, h - 1))],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        time.sleep(0.08)
        subprocess.call(
            ["xdotool", "windowmove", wid, str(x), str(y), "windowsize", wid, str(w), str(h)],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        time.sleep(0.08)
        subprocess.call(
            ["xdotool", "windowstate", "--add", "FULLSCREEN", wid],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        subprocess.call(["xdotool", "windowactivate", wid], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        print(f"Blightnet window {wid} → {name} {w}x{h} at {x},{y}", flush=True)
    except Exception as exc:
        print(f"Blightnet window move failed ({exc})", flush=True)
        return None
    return rec


def _chrome_binaries() -> list[str]:
    names = (
        "google-chrome-stable",
        "google-chrome",
        "chromium",
        "chromium-browser",
        "microsoft-edge-stable",
        "microsoft-edge",
        "brave-browser",
        "brave",
    )
    found: list[str] = []
    for name in names:
        path = shutil.which(name)
        if path and path not in found:
            found.append(path)
    return found


def _reap_profile_browsers() -> None:
    """An existing Brave with this profile swallows --app= and exits immediately."""
    needle = CHROME_PROFILE.encode()
    pids = []
    for pid in os.listdir("/proc"):
        if not pid.isdigit():
            continue
        try:
            cmd = open(f"/proc/{pid}/cmdline", "rb").read()
        except Exception:
            continue
        if needle in cmd and b"--app=" in cmd:
            pids.append(int(pid))
    for pid in pids:
        try:
            os.kill(pid, signal.SIGTERM)
        except OSError:
            pass
    if pids:
        time.sleep(0.35)
        for pid in pids:
            try:
                os.kill(pid, signal.SIGKILL)
            except OSError:
                pass


def _run_gpu_app_window(url: str) -> int | None:
    """Chromium --app= window. Same GPU path as Brave; WebKitGTK caps around 18 fps here."""
    global _window_proc
    bins = _chrome_binaries()
    if not bins:
        return None
    os.makedirs(CHROME_PROFILE, exist_ok=True)
    _reap_profile_browsers()
    env = os.environ.copy()
    env.pop("PULSE_SINK", None)
    env.pop("PIPEWIRE_NODE", None)
    env.pop("BLIGHNET_AUDIO_TARGET", None)
    geo = None
    pref = load_display_pref()
    if pref:
        geo = next((d for d in list_displays() if d["name"] == pref), None)
    if geo is None:
        shown = list_displays()
        geo = next((d for d in shown if d.get("on")), shown[0] if shown else None)
    pos = f"{geo['x']},{geo['y']}" if geo else "0,0"
    size = f"{geo['w']},{geo['h']}" if geo else "1920,1080"
    flags = [
        "--app=" + url,
        "--user-data-dir=" + CHROME_PROFILE,
        "--class=Blightnet",
        "--name=Blightnet",
        "--no-first-run",
        "--no-default-browser-check",
        "--disable-sync",
        "--disable-features=TranslateUI,CalculateNativeWinOcclusion",
        "--disable-session-crashed-bubble",
        "--hide-crash-restore-bubble",
        "--disable-background-timer-throttling",
        "--disable-backgrounding-occluded-windows",
        "--disable-renderer-backgrounding",
        "--enable-gpu-rasterization",
        "--enable-zero-copy",
        "--ignore-gpu-blocklist",
        "--start-fullscreen",
        "--window-position=" + pos,
        "--window-size=" + size,
        "--password-store=basic",
        "--autoplay-policy=no-user-gesture-required",
    ]
    if os.environ.get("WAYLAND_DISPLAY") and os.environ.get("XDG_SESSION_TYPE") == "wayland":
        flags.append("--ozone-platform=x11")
    for binary in bins:
        cmd = [binary, *flags]
        print(f"Blightnet window → {binary} --app (GPU)", flush=True)
        try:
            proc = subprocess.Popen(cmd, env=env)
        except Exception as exc:
            print(f"Blightnet GPU window skipped ({binary}: {exc}).", flush=True)
            continue
        _window_proc = proc
        print(f"Blightnet window pid {proc.pid}", flush=True)
        try:
            return int(proc.wait())
        except KeyboardInterrupt:
            proc.terminate()
            try:
                proc.wait(timeout=3)
            except Exception:
                proc.kill()
            return 0
    return None


def open_system_browser(url: str) -> None:
    cmds: list[list[str]] = []
    if sys.platform == "win32":
        cmds = [
            ["cmd", "/C", "start", "", "msedge", "--app=" + url],
            ["cmd", "/C", "start", "", "chrome", "--app=" + url],
        ]
    elif sys.platform == "darwin":
        cmds = [["open", "-a", "Google Chrome", "--args", "--app=" + url]]
    else:
        for binary in (
            "google-chrome",
            "google-chrome-stable",
            "chromium",
            "chromium-browser",
            "microsoft-edge",
            "microsoft-edge-stable",
        ):
            path = shutil.which(binary)
            if not path:
                continue
            cmds.append([path, "--app=" + url, "--new-window"])
    for cmd in cmds:
        try:
            subprocess.Popen(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            return
        except Exception:
            continue
    webbrowser.open(url)


def _run_blightnet_window(url: str, extra_env: dict | None = None) -> int:
    global _window_proc
    script = os.path.join(ROOT, "blightnet_window.py")
    env = os.environ.copy()
    gst_dir = os.path.join(ROOT, "gst")
    if os.path.isdir(gst_dir):
        old = env.get("GST_PLUGIN_PATH", "")
        if gst_dir not in old.split(":"):
            env["GST_PLUGIN_PATH"] = gst_dir + ((":" + old) if old else "")
    if extra_env:
        env.update(extra_env)
    if env.get("GDK_BACKEND") == "wayland":
        env["WEBKIT_DISABLE_COMPOSITING_MODE"] = "1"
        env.pop("WEBKIT_DISABLE_DMABUF_RENDERER", None)
    else:
        env.pop("WEBKIT_DISABLE_COMPOSITING_MODE", None)
        env.setdefault("WEBKIT_DISABLE_DMABUF_RENDERER", "1")
    backend = env.get("GDK_BACKEND") or "default"
    print(f"Blightnet window → {script} ({backend})", flush=True)
    proc = subprocess.Popen([sys.executable, "-u", script, url], env=env)
    _window_proc = proc
    print(f"Blightnet window pid {proc.pid}", flush=True)
    try:
        return int(proc.wait())
    except KeyboardInterrupt:
        proc.terminate()
        try:
            proc.wait(timeout=3)
        except Exception:
            proc.kill()
        return 0


def open_native_window(url: str) -> bool:
    if not sys.platform.startswith("linux"):
        return False
    gpu_rc = _run_gpu_app_window(url)
    if gpu_rc is not None:
        return gpu_rc == 0
    script = os.path.join(ROOT, "blightnet_window.py")
    if not os.path.isfile(script):
        print("Blightnet window: blightnet_window.py is missing.", flush=True)
        return False
    # WebKitGTK on native Wayland (KDE) has crashed with protocol error 71.
    first_env = None
    if os.environ.get("WAYLAND_DISPLAY") and os.environ.get("GDK_BACKEND") != "x11":
        first_env = {"GDK_BACKEND": "x11"}
    rc = _run_blightnet_window(url, first_env)
    if rc != 0 and first_env:
        print("Blightnet window failed on X11. Trying native Wayland…", flush=True)
        rc = _run_blightnet_window(url, {"GDK_BACKEND": "wayland"})
    if rc != 0:
        print("Blightnet window could not stay open.", flush=True)
        return False
    return True


def main() -> int:
    parser = argparse.ArgumentParser(description="Serve Blightnet locally")
    parser.add_argument("--port", type=int, default=PORT_DEFAULT)
    parser.add_argument("--no-open", action="store_true", help="Do not open a browser")
    parser.add_argument("--system-browser", action="store_true", help="Open the system browser instead of the Blightnet window")
    args = parser.parse_args()

    os.chdir(ROOT)
    os.makedirs(UPLOAD_DIR, exist_ok=True)
    reclaim_port(args.port)
    port = pick_port(args.port)
    if port != args.port:
        print(f"Port {args.port} is busy. Using {port} instead.", flush=True)
    global _httpd
    handler = functools.partial(Handler, directory=ROOT)
    try:
        httpd = Server(("::", port), handler)
    except OSError:
        Server.address_family = socket.AF_INET
        httpd = Server(("0.0.0.0", port), handler)
    threading.Thread(target=punch_internet, args=(port,), daemon=True).start()
    _httpd = httpd
    url = f"http://127.0.0.1:{port}/"
    print(f"Blightnet → {url}", flush=True)
    for ip in lan_ips():
        print(f"  table  → http://{ip}:{port}/", flush=True)

    if args.no_open:
        try:
            httpd.serve_forever()
        except KeyboardInterrupt:
            print("\nStopped.")
        finally:
            httpd.server_close()
        return 0

    worker = threading.Thread(target=httpd.serve_forever, daemon=True)
    worker.start()
    time.sleep(0.45)
    if args.system_browser:
        print("Opening the system browser (--system-browser).", flush=True)
        try:
            open_system_browser(url)
        except Exception as exc:
            print(f"Open this URL yourself: {url} ({exc})", flush=True)
        try:
            httpd.serve_forever()
        except KeyboardInterrupt:
            print("\nStopped.")
        finally:
            httpd.shutdown()
            httpd.server_close()
        return 0

    print("Opening the Blightnet window…", flush=True)
    native = open_native_window(url)
    if not native and sys.platform == "win32":
        print("Opening the table as an app window…", flush=True)
        try:
            open_system_browser(url)
        except Exception as exc:
            print(f"Open this URL yourself: {url} ({exc})", flush=True)
        try:
            httpd.serve_forever()
        except KeyboardInterrupt:
            print("\nStopped.")
        finally:
            httpd.shutdown()
            httpd.server_close()
        return 0
    if not native:
        print(
            "The standalone window did not open.\n"
            f"  Leave this running and, only if you must, open {url} yourself.\n"
            "  Or install WebKitGTK (webkit2gtk) and try again.\n"
            "  Firefox/Brave will not be launched automatically.",
            flush=True,
        )
        try:
            httpd.serve_forever()
        except KeyboardInterrupt:
            print("\nStopped.")
    httpd.shutdown()
    httpd.server_close()
    return 0


if __name__ == "__main__":
    sys.exit(main())
