const CHUNK = 16 * 1024;

function peerId() {
  let id = localStorage.getItem("hearthsong.peerId");
  if (!id) {
    id = "p-" + Math.random().toString(36).slice(2, 10) + Date.now().toString(36).slice(-4);
    localStorage.setItem("hearthsong.peerId", id);
  }
  return id;
}

export function profileName() {
  const raw = (localStorage.getItem("hearthsong.profileName") || "").trim();
  return raw.slice(0, 24) || "Traveller";
}

export function setProfileName(name) {
  const next = String(name || "").trim().slice(0, 24) || "Traveller";
  localStorage.setItem("hearthsong.profileName", next);
  return next;
}

export function profilePic() {
  try {
    const raw = localStorage.getItem("hearthsong.profilePic") || "";
    return raw.startsWith("data:image/") ? raw : "";
  } catch {
    return "";
  }
}

export function setProfilePic(dataUrl) {
  const next = String(dataUrl || "");
  try {
    if (next.startsWith("data:image/") && next.length < 120000) {
      localStorage.setItem("hearthsong.profilePic", next);
    } else {
      localStorage.removeItem("hearthsong.profilePic");
    }
  } catch {
    /* ignore quota */
  }
  return profilePic();
}

function parseWhisperLine(line, peers, selfId) {
  const m = String(line || "").match(/^\/(?:w|whisper|msg|tell)(?:\s+|$)(.*)$/i);
  if (!m) return { whisper: false };
  const rest = (m[1] || "").trim();
  if (!rest) {
    return {
      whisper: true,
      localOnly: true,
      error: "Whisper with /w Name then your message, or click a name above.",
    };
  }
  const others = (peers || []).filter((p) => p.id !== selfId && p.name);
  let name = "";
  let body = rest;
  if (rest[0] === '"' || rest[0] === "'") {
    const q = rest[0];
    const end = rest.indexOf(q, 1);
    if (end < 0) {
      return {
        whisper: true,
        localOnly: true,
        error: "Close the quotes around the handle, then type your whisper.",
      };
    }
    name = rest.slice(1, end).trim();
    body = rest.slice(end + 1).trim();
  } else {
    const lower = rest.toLowerCase();
    let best = null;
    let bestLen = -1;
    for (const p of others) {
      const n = String(p.name).trim();
      const nl = n.toLowerCase();
      if (lower === nl || lower.startsWith(nl + " ")) {
        if (n.length > bestLen) {
          best = p;
          bestLen = n.length;
        }
      }
    }
    if (best) {
      body = rest.slice(bestLen).trim();
      if (!body) {
        return { whisper: true, localOnly: true, error: "Type a whisper after the name." };
      }
      return { whisper: true, to: best.id, toName: best.name, body };
    }
    const sp = rest.match(/^(\S+)\s*(.*)$/);
    name = sp ? sp[1] : rest;
    body = sp ? sp[2].trim() : "";
  }
  const want = name.trim().toLowerCase();
  const exact = others.filter((p) => String(p.name).trim().toLowerCase() === want);
  const prefix = others.filter((p) => String(p.name).trim().toLowerCase().startsWith(want));
  let hit = exact.length === 1 ? exact[0] : prefix.length === 1 ? prefix[0] : null;
  if (exact.length > 1 || (!hit && prefix.length > 1)) {
    return { whisper: true, error: "Several players match " + name + ". Use the full handle." };
  }
  if (!hit) return { whisper: true };
  if (!body) {
    return { whisper: true, localOnly: true, error: "Type a whisper after the name." };
  }
  return { whisper: true, to: hit.id, toName: hit.name, body };
}

export function createTable(hooks) {
  const state = {
    role: "idle",
    hostUrl: "",
    ws: null,
    peers: [],
    files: [],
    pcs: new Map(),
    dcs: new Map(),
    incoming: new Map(),
    pendingIce: new Map(),
    sendQ: new Map(),
    pendingOut: new Map(),
    pendingNeeds: new Set(),
    pingTimer: 0,
    selfId: peerId(),
    mic: null,
    tableVoice: false,
    callId: "",
    pendingCall: "",
    incomingCall: null,
    audios: new Map(),
    renegotiating: new Set(),
  };

  function status(text, extra) {
    hooks.onStatus?.(text, extra || {});
  }

  function send(msg) {
    if (state.ws && state.ws.readyState === WebSocket.OPEN) {
      state.ws.send(JSON.stringify(msg));
    }
  }

  function wsUrl(httpUrl) {
    const u = new URL(httpUrl, location.href);
    u.protocol = u.protocol === "https:" ? "wss:" : "ws:";
    u.pathname = "/ws";
    u.search = "";
    u.hash = "";
    return u.toString();
  }

  function originOf(httpUrl) {
    const u = new URL(httpUrl, location.href);
    return u.origin;
  }

  function closeAllRtc() {
    for (const dc of state.dcs.values()) {
      try {
        dc.close();
      } catch {
        /* ignore */
      }
    }
    for (const pc of state.pcs.values()) {
      try {
        pc.close();
      } catch {
        /* ignore */
      }
    }
    for (const id of [...state.audios.keys()]) clearRemoteAudio(id);
    state.pcs.clear();
    state.dcs.clear();
    state.incoming.clear();
  }

  function clearRemoteAudio(id) {
    const el = state.audios.get(id);
    if (!el) return;
    try {
      el.pause();
    } catch {
      /* ignore */
    }
    el.srcObject = null;
    el.remove();
    state.audios.delete(id);
  }

  function stopMic() {
    if (!state.mic) return;
    for (const track of state.mic.getTracks()) {
      try {
        track.stop();
      } catch {
        /* ignore */
      }
    }
    state.mic = null;
  }

  function micTrack() {
    return state.mic?.getAudioTracks().find((t) => t.readyState === "live") || null;
  }

  function wantsSend(remoteId) {
    if (!micTrack()) return false;
    if (state.callId) return remoteId === state.callId;
    if (state.pendingCall || state.incomingCall) return false;
    return Boolean(state.tableVoice);
  }

  function wantsHear(remoteId) {
    if (state.callId) return remoteId === state.callId;
    if (state.pendingCall || state.incomingCall) return false;
    return Boolean(state.tableVoice);
  }

  function voiceSnapshot() {
    return {
      table: state.tableVoice,
      callId: state.callId,
      pendingCall: state.pendingCall,
      incoming: state.incomingCall,
      live: Boolean(micTrack()),
    };
  }

  function emitVoice() {
    hooks.onVoice?.(voiceSnapshot());
  }

  function shouldOffer(remoteId) {
    if (state.role === "host") return true;
    const host = state.peers.find((p) => p.role === "host");
    if (host && remoteId === host.id) return false;
    return state.selfId < remoteId;
  }

  function meshPeers() {
    if (state.role === "idle") return;
    for (const peer of state.peers) {
      if (!peer?.id || peer.id === state.selfId) continue;
      if (state.pcs.has(peer.id)) continue;
      if (shouldOffer(peer.id)) callPeer(peer.id).catch(() => {});
    }
  }

  async function enableMic() {
    const live = micTrack();
    if (live) return;
    if (!navigator.mediaDevices?.getUserMedia) {
      const err = new Error("secure");
      err.code = "secure";
      throw err;
    }
    try {
      state.mic = await navigator.mediaDevices.getUserMedia({
        audio: { echoCancellation: true, noiseSuppression: true, autoGainControl: true },
        video: false,
      });
    } catch (e) {
      const err = new Error(e?.name === "NotAllowedError" ? "denied" : "secure");
      err.code = e?.name === "NotAllowedError" ? "denied" : "secure";
      throw err;
    }
  }

  function attachRemoteAudio(remoteId, track) {
    let el = state.audios.get(remoteId);
    if (!el) {
      el = document.createElement("audio");
      el.autoplay = true;
      el.setAttribute("playsinline", "");
      el.setAttribute("data-voice", remoteId);
      (document.getElementById("voice-sinks") || document.body).append(el);
      state.audios.set(remoteId, el);
    }
    const stream = el.srcObject instanceof MediaStream ? el.srcObject : new MediaStream();
    for (const old of stream.getAudioTracks()) {
      if (old !== track) stream.removeTrack(old);
    }
    if (!stream.getTracks().includes(track)) stream.addTrack(track);
    el.srcObject = stream;
    el.muted = !wantsHear(remoteId);
    el.volume = 1;
    const play = () => el.play().catch(() => {});
    play();
    track.onended = () => {
      if (el.srcObject === stream) {
        stream.removeTrack(track);
        if (!stream.getTracks().length) clearRemoteAudio(remoteId);
      }
    };
  }

  async function renegotiate(remoteId) {
    const pc = state.pcs.get(remoteId);
    if (!pc || pc.signalingState !== "stable") return;
    if (state.renegotiating.has(remoteId)) return;
    state.renegotiating.add(remoteId);
    try {
      const offer = await pc.createOffer();
      if (pc.signalingState !== "stable") return;
      await pc.setLocalDescription(offer);
      send({ type: "signal", to: remoteId, payload: { sdp: pc.localDescription } });
    } catch {
      /* ignore */
    } finally {
      state.renegotiating.delete(remoteId);
    }
  }

  async function syncVoiceTo(remoteId) {
    const pc = state.pcs.get(remoteId);
    if (!pc) return;
    const track = micTrack();
    const want = Boolean(track && wantsSend(remoteId));
    const sender = pc.getSenders().find((s) => s.dtmf || s.track?.kind === "audio");
    try {
      if (want) {
        if (sender) {
          if (sender.track !== track) await sender.replaceTrack(track);
        } else {
          pc.addTrack(track, state.mic);
          await renegotiate(remoteId);
        }
      } else if (sender?.track) {
        await sender.replaceTrack(null);
      }
    } catch {
      /* ignore */
    }
    const el = state.audios.get(remoteId);
    if (el) el.muted = !wantsHear(remoteId);
  }

  async function syncAllVoice() {
    await Promise.all([...state.pcs.keys()].map((id) => syncVoiceTo(id)));
    emitVoice();
  }

  function disconnect() {
    const was = state.role;
    state.role = "idle";
    state.hostUrl = "";
    state.peers = [];
    closeAllRtc();
    if (state.pingTimer) {
      window.clearInterval(state.pingTimer);
      state.pingTimer = 0;
    }
    if (state.ws) {
      try {
        state.ws.close();
      } catch {
        /* ignore */
      }
      state.ws = null;
    }
    state.tableVoice = false;
    state.callId = "";
    state.pendingCall = "";
    state.incomingCall = null;
    stopMic();
    if (was !== "idle") status("Offline");
    hooks.onPeers?.([]);
    emitVoice();
  }

  function ensurePc(remoteId) {
    let pc = state.pcs.get(remoteId);
    if (pc) return pc;
    pc = new RTCPeerConnection({
      iceServers: [
        { urls: ["stun:stun.l.google.com:19302", "stun:stun1.l.google.com:19302"] },
        { urls: "stun:stun.cloudflare.com:3478" },
      ],
      iceCandidatePoolSize: 4,
    });
    pc.onicecandidate = (ev) => {
      if (ev.candidate) {
        send({ type: "signal", to: remoteId, payload: { candidate: ev.candidate } });
      }
    };
    pc.onconnectionstatechange = () => {
      if (pc.connectionState === "failed" && state.role === "host") {
        state.pcs.delete(remoteId);
        state.dcs.delete(remoteId);
        callPeer(remoteId).catch(() => {});
        return;
      }
      if (pc.connectionState === "failed" || pc.connectionState === "closed") {
        try {
          pc.close();
        } catch {
          /* ignore */
        }
        state.pcs.delete(remoteId);
        state.dcs.delete(remoteId);
        state.pendingIce.delete(remoteId);
        clearRemoteAudio(remoteId);
      }
    };
    pc.ondatachannel = (ev) => bindDc(remoteId, ev.channel);
    try {
      pc.addTransceiver("audio", { direction: "sendrecv" });
    } catch {
      /* older engines still get audio via addTrack */
    }
    pc.ontrack = (ev) => {
      const track = ev.track;
      if (!track || track.kind !== "audio") return;
      attachRemoteAudio(remoteId, track);
    };
    state.pcs.set(remoteId, pc);
    syncVoiceTo(remoteId).catch(() => {});
    return pc;
  }

  function bindDc(remoteId, dc) {
    dc.binaryType = "arraybuffer";
    state.dcs.set(remoteId, dc);
    dc.onopen = () => {
      const queued = state.pendingOut.get(remoteId) || [];
      state.pendingOut.delete(remoteId);
      for (const rec of queued) sendFileTo(remoteId, rec);
      if (state.role === "host") hooks.onNeedPush?.(remoteId);
      flushNeeds();
    };
    dc.onmessage = (ev) => onDcMessage(remoteId, ev.data);
  }

  async function callPeer(remoteId) {
    const pc = ensurePc(remoteId);
    if (state.dcs.get(remoteId)?.readyState === "open") {
      await syncVoiceTo(remoteId);
      return;
    }
    if (!state.dcs.has(remoteId)) {
      const dc = pc.createDataChannel("hearth");
      bindDc(remoteId, dc);
    }
    await syncVoiceTo(remoteId);
    if (pc.signalingState !== "stable") return;
    const offer = await pc.createOffer();
    await pc.setLocalDescription(offer);
    send({ type: "signal", to: remoteId, payload: { sdp: pc.localDescription } });
  }

  function sendVoice(action, to) {
    send({ type: "voice", action, to });
  }

  function onVoiceMessage(msg) {
    const from = msg.from;
    const action = msg.action;
    if (!from || from === state.selfId) return;
    if (action === "invite") {
      if (state.callId || state.pendingCall || state.incomingCall) {
        sendVoice("decline", from);
        return;
      }
      state.incomingCall = { from, name: msg.name || "Traveller" };
      emitVoice();
      return;
    }
    if (action === "accept") {
      if (state.pendingCall !== from) return;
      state.pendingCall = "";
      state.callId = from;
      state.incomingCall = null;
      meshPeers();
      syncAllVoice().catch(() => {});
      return;
    }
    if (action === "decline") {
      if (state.pendingCall === from) state.pendingCall = "";
      if (state.incomingCall?.from === from) state.incomingCall = null;
      emitVoice();
      return;
    }
    if (action === "hangup") {
      if (state.callId === from || state.pendingCall === from || state.incomingCall?.from === from) {
        state.callId = "";
        state.pendingCall = "";
        state.incomingCall = null;
        if (!state.tableVoice) stopMic();
        syncAllVoice().catch(() => {});
      }
    }
  }

  async function onSignal(from, payload) {
    if (!payload || from === state.selfId) return;
    const pc = ensurePc(from);
    if (payload.sdp) {
      const desc = payload.sdp;
      await pc.setRemoteDescription(new RTCSessionDescription(desc));
      const queued = state.pendingIce.get(from) || [];
      state.pendingIce.delete(from);
      for (const c of queued) {
        try {
          await pc.addIceCandidate(new RTCIceCandidate(c));
        } catch {
          /* ignore */
        }
      }
      if (desc.type === "offer") {
        const answer = await pc.createAnswer();
        await pc.setLocalDescription(answer);
        send({ type: "signal", to: from, payload: { sdp: pc.localDescription } });
      }
    }
    if (payload.candidate) {
      if (!pc.remoteDescription) {
        const q = state.pendingIce.get(from) || [];
        q.push(payload.candidate);
        state.pendingIce.set(from, q);
      } else {
        try {
          await pc.addIceCandidate(new RTCIceCandidate(payload.candidate));
        } catch {
          /* ignore */
        }
      }
    }
  }

  function onDcMessage(remoteId, data) {
    if (typeof data === "string") {
      let msg;
      try {
        msg = JSON.parse(data);
      } catch {
        return;
      }
      if (msg.type === "file-start") {
        state.incoming.set(msg.id, {
          meta: msg,
          chunks: [],
          got: 0,
          from: remoteId,
        });
      } else if (msg.type === "file-end") {
        const rec = state.incoming.get(msg.id);
        if (!rec) return;
        const blob = new Blob(rec.chunks, { type: rec.meta.mime || "application/octet-stream" });
        state.incoming.delete(msg.id);
        state.pendingNeeds.delete(msg.id);
        hooks.onFile?.({ ...rec.meta, blob, from: remoteId });
      } else if (msg.type === "need-file") {
        hooks.onNeedFile?.(remoteId, msg.id);
      }
      return;
    }
    const incoming = [...state.incoming.values()].find((r) => r.from === remoteId && r.got < (r.meta.size || Infinity));
    if (!incoming) return;
    incoming.chunks.push(data);
    incoming.got += data.byteLength || data.size || 0;
  }

  async function waitDrain(dc) {
    while (dc.bufferedAmount > CHUNK * 8) {
      await new Promise((r) => setTimeout(r, 20));
    }
  }

  function flushNeeds() {
    if (!state.pendingNeeds.size) return;
    for (const dc of state.dcs.values()) {
      if (dc.readyState !== "open") continue;
      for (const id of state.pendingNeeds) {
        try {
          dc.send(JSON.stringify({ type: "need-file", id }));
        } catch {
          /* channel busy */
        }
      }
    }
  }

  async function sendFileTo(remoteId, rec) {
    const dc = state.dcs.get(remoteId);
    if (!dc || dc.readyState !== "open" || !rec?.bytes) {
      if (rec?.bytes) {
        const q = state.pendingOut.get(remoteId) || [];
        q.push(rec);
        state.pendingOut.set(remoteId, q);
      }
      return;
    }
    const prev = state.sendQ.get(remoteId) || Promise.resolve();
    const next = prev.catch(() => {}).then(async () => {
      if (dc.readyState !== "open") return;
      dc.send(
        JSON.stringify({
          type: "file-start",
          id: rec.id,
          name: rec.name,
          category: rec.category,
          kind: rec.kind || rec.category,
          mime: rec.mime,
          size: rec.bytes.byteLength,
          icon: rec.icon,
          mood: rec.mood,
          w: rec.w,
          h: rec.h,
          chat: Boolean(rec.chat),
          whisper: Boolean(rec.whisper),
          to: rec.to || "",
          toName: rec.toName || "",
          fromName: rec.fromName || "",
        })
      );
      const buf = rec.bytes;
      for (let i = 0; i < buf.byteLength; i += CHUNK) {
        await waitDrain(dc);
        if (dc.readyState !== "open") return;
        dc.send(buf.slice(i, i + CHUNK));
      }
      if (dc.readyState === "open") dc.send(JSON.stringify({ type: "file-end", id: rec.id }));
    });
    state.sendQ.set(remoteId, next);
    return next;
  }

  function connect(httpUrl, role) {
    disconnect();
    state.role = role;
    state.hostUrl = httpUrl.replace(/\/+$/, "");
    status(role === "host" ? "Opening table…" : "Joining table…");
    const socket = new WebSocket(wsUrl(state.hostUrl));
    state.ws = socket;
    socket.onopen = () => {
      send({
        type: "hello",
        id: state.selfId,
        name: profileName(),
        role,
        pic: profilePic(),
      });
      if (state.pingTimer) window.clearInterval(state.pingTimer);
      state.pingTimer = window.setInterval(() => {
        if (state.ws === socket && socket.readyState === WebSocket.OPEN) send({ type: "ping" });
      }, 20000);
    };
    socket.onclose = () => {
      if (state.pingTimer) {
        window.clearInterval(state.pingTimer);
        state.pingTimer = 0;
      }
      if (state.ws === socket) {
        state.ws = null;
        if (state.role !== "idle") {
          state.role = "idle";
          closeAllRtc();
          status("Offline");
          hooks.onPeers?.([]);
        }
      }
    };
    socket.onerror = () => {
      status("Could not reach the table. Check the address and that they are online.");
    };
    let inbound = Promise.resolve();
    socket.onmessage = (ev) => {
      inbound = inbound.catch(() => {}).then(() => onWsMessage(ev));
    };
    async function onWsMessage(ev) {
      let msg;
      try {
        msg = JSON.parse(ev.data);
      } catch {
        return;
      }
      if (msg.type === "pong" || msg.type === "ping") return;
      if (msg.type === "welcome") {
        if (msg.id) state.selfId = msg.id;
        if (msg.role) state.role = msg.role;
        state.peers = msg.peers || [];
        hooks.onPeers?.(state.peers);
        if (msg.files?.length) await hooks.onFileList?.(msg.files, originOf(state.hostUrl));
        if (msg.mix) hooks.onMix?.(msg.mix);
        if (msg.map) hooks.onMap?.(msg.map);
        if (msg.chars) hooks.onCharsTable?.(msg.chars);
        if (msg.rolls?.length) hooks.onRolls?.(msg.rolls);
        if (msg.pics?.length) hooks.onProfiles?.(msg.pics);
        status(state.role === "host" ? "Hosting" : "Joined", { hosting: state.role === "host" });
        meshPeers();
        hooks.onReady?.();
      } else if (msg.type === "peers") {
        state.peers = msg.peers || [];
        hooks.onPeers?.(state.peers);
        meshPeers();
        if (state.callId && !state.peers.some((p) => p.id === state.callId)) {
          state.callId = "";
          if (!state.tableVoice) stopMic();
          syncAllVoice().catch(() => {});
        }
        if (state.incomingCall && !state.peers.some((p) => p.id === state.incomingCall.from)) {
          state.incomingCall = null;
          emitVoice();
        }
      } else if (msg.type === "voice") {
        onVoiceMessage(msg);
      } else if (msg.type === "chat") {
        hooks.onChat?.(msg);
      } else if (msg.type === "mix") {
        if (state.role !== "host") hooks.onMix?.(msg.mix);
      } else if (msg.type === "map") {
        if (state.role !== "host") hooks.onMap?.(msg.map);
      } else if (msg.type === "map-edit") {
        if (state.role === "host") hooks.onMapEdit?.(msg.edit, msg.from);
      } else if (msg.type === "files") {
        hooks.onFileList?.(msg.files || [], originOf(state.hostUrl));
      } else if (msg.type === "chars") {
        hooks.onChars?.(msg.from, msg.name, msg.list);
      } else if (msg.type === "roll" || msg.type === "log") {
        if (msg.from !== state.selfId) hooks.onRoll?.(msg);
      } else if (msg.type === "profile") {
        if (msg.from !== state.selfId) hooks.onProfile?.(msg.from, msg.name, msg.pic || "");
      } else if (msg.type === "signal") {
        await onSignal(msg.from, msg.payload);
      }
    }
  }

  return {
    state,
    send,
    connect,
    disconnect,
    sendFileTo,
    async sendChatMedia(file, extra = {}) {
      if (state.role === "idle") return { error: "Host or join a table to send pictures." };
      if (!file) return { error: "Pick a picture or video." };
      const mime = String(file.type || "").toLowerCase();
      const image = mime.startsWith("image/");
      const video = mime.startsWith("video/");
      if (!image && !video) return { error: "Send a picture or a video." };
      const max = video ? 40 * 1024 * 1024 : 12 * 1024 * 1024;
      if (file.size > max) {
        return { error: video ? "That video is too large (max 40 MB)." : "That picture is too large (max 12 MB)." };
      }
      let to = String(extra.to || "").trim();
      let toName = String(extra.toName || "").trim();
      let whisper = Boolean(extra.whisper || to);
      if (whisper && !to) {
        const parsed = parseWhisperLine("/w " + (toName || extra.hint || ""), state.peers, state.selfId);
        if (parsed.to) {
          to = parsed.to;
          toName = parsed.toName || toName;
        } else if (parsed.error) {
          return { error: parsed.error };
        } else {
          return { error: "No one by that name is at the table." };
        }
      }
      if (whisper && to && to === state.selfId) return { error: "Send that to someone else." };
      if (whisper && to && !state.peers.some((p) => p.id === to)) {
        return { error: "They are not at this table." };
      }
      const targets = whisper
        ? [to]
        : state.peers.filter((p) => p.id !== state.selfId).map((p) => p.id);
      if (!targets.length) return { error: "No one else is at the table." };
      meshPeers();
      const bytes = await file.arrayBuffer();
      const rec = {
        id: "media-" + Date.now().toString(36) + Math.random().toString(36).slice(2, 6),
        name: String(file.name || (image ? "picture" : "video")).slice(0, 80),
        kind: "chat-media",
        category: "chat-media",
        mime: mime || (image ? "image/jpeg" : "video/mp4"),
        bytes,
        chat: true,
        whisper,
        to,
        toName,
        fromName: profileName(),
      };
      await Promise.all(targets.map((id) => sendFileTo(id, rec)));
      return { ok: true, rec, whisper, to, toName };
    },
    origin: () => (state.hostUrl ? originOf(state.hostUrl) : location.origin),
    chat(text) {
      const line = String(text || "").trim().slice(0, 400);
      if (!line) return;
      const parsed = parseWhisperLine(line, state.peers, state.selfId);
      if (parsed.whisper) {
        if (parsed.localOnly) {
          hooks.onChat?.({ sys: true, text: parsed.error });
          return;
        }
        if (parsed.to && parsed.body) {
          send({ type: "chat", text: parsed.body, to: parsed.to, whisper: true });
          return;
        }
        send({ type: "chat", text: line, whisper: true });
        return;
      }
      send({ type: "chat", text: line });
    },
    sendMix(mix) {
      if (state.role !== "host" || !mix) return;
      const payload = { ...mix };
      delete payload.master;
      send({ type: "mix", mix: payload });
    },
    sendMap(map) {
      if (state.role === "host") send({ type: "map", map });
    },
    sendMapEdit(edit) {
      if (state.role === "guest") send({ type: "map-edit", edit });
    },
    sendChars(list) {
      if (state.role === "idle") return;
      send({ type: "chars", list: Array.isArray(list) ? list : [] });
    },
    sendRoll(row) {
      if (state.role === "idle" || !row) return;
      if (row.info) {
        send({
          type: "log",
          who: String(row.who || "").slice(0, 48),
          action: String(row.action || "look").slice(0, 80),
          text: String(row.text || "").slice(0, 800),
          player: String(row.player || profileName()).slice(0, 24),
        });
        return;
      }
      const pct = Math.max(0, Math.min(100, Number(row.pct)));
      const dmg = row.damage == null || row.damage === "" ? null : Number(row.damage);
      send({
        type: "roll",
        who: String(row.who || "").slice(0, 48),
        action: String(row.action || "roll").slice(0, 80),
        pct: Number.isFinite(pct) ? pct : 0,
        grade: String(row.grade || "").slice(0, 24),
        damage: Number.isFinite(dmg) ? dmg : null,
        taken: Boolean(row.taken),
        heal: Boolean(row.heal),
        player: String(row.player || profileName()).slice(0, 24),
        hack: Boolean(row.hack),
        deathSave: Boolean(row.deathSave),
        targetId: String(row.targetId || "").slice(0, 64),
        targetName: String(row.targetName || "").slice(0, 48),
        targetOwner: String(row.targetOwner || "").slice(0, 24),
        tokenId: String(row.tokenId || "").slice(0, 32),
        sheetId: String(row.sheetId || "").slice(0, 64),
        sheetOwner: String(row.sheetOwner || "").slice(0, 24),
        adv: row.adv === "adv" || row.adv === "dis" ? row.adv : "",
        pctA: Number.isFinite(Number(row.pctA)) ? Math.max(0, Math.min(100, Number(row.pctA))) : null,
        pctB: Number.isFinite(Number(row.pctB)) ? Math.max(0, Math.min(100, Number(row.pctB))) : null,
      });
    },
    requestFile(id) {
      const want = String(id || "");
      if (!want) return;
      state.pendingNeeds.add(want);
      flushNeeds();
    },
    rename(name) {
      if (state.ws && state.ws.readyState === WebSocket.OPEN) {
        send({ type: "hello", id: state.selfId, name, role: state.role === "host" ? "host" : "guest", pic: profilePic() });
      }
    },
    sendProfile(pic) {
      if (state.role === "idle") return;
      send({ type: "profile", pic: String(pic || "") });
    },
    voice: voiceSnapshot,
    async setTableVoice(on) {
      if (state.role === "idle") throw Object.assign(new Error("idle"), { code: "idle" });
      if (on) {
        await enableMic();
        state.tableVoice = true;
        meshPeers();
        if (!state.callId && !state.pendingCall && !state.incomingCall) await syncAllVoice();
        else emitVoice();
      } else {
        state.tableVoice = false;
        if (!state.callId && !state.pendingCall) stopMic();
        await syncAllVoice();
      }
    },
    async startCall(peerId) {
      if (state.role === "idle") throw Object.assign(new Error("idle"), { code: "idle" });
      const id = String(peerId || "");
      if (!id || id === state.selfId) throw Object.assign(new Error("self"), { code: "self" });
      if (!state.peers.some((p) => p.id === id)) throw Object.assign(new Error("offline"), { code: "offline" });
      if (state.callId || state.pendingCall) throw Object.assign(new Error("busy"), { code: "busy" });
      await enableMic();
      if (state.incomingCall) {
        sendVoice("decline", state.incomingCall.from);
        state.incomingCall = null;
      }
      state.pendingCall = id;
      sendVoice("invite", id);
      meshPeers();
      emitVoice();
    },
    async acceptCall() {
      const incoming = state.incomingCall;
      if (!incoming) return;
      await enableMic();
      const from = incoming.from;
      state.incomingCall = null;
      state.pendingCall = "";
      state.callId = from;
      sendVoice("accept", from);
      meshPeers();
      await syncAllVoice();
    },
    declineCall() {
      const incoming = state.incomingCall;
      if (!incoming) return;
      sendVoice("decline", incoming.from);
      state.incomingCall = null;
      emitVoice();
    },
    async hangup() {
      const other = state.callId || state.pendingCall;
      if (other) sendVoice("hangup", other);
      state.callId = "";
      state.pendingCall = "";
      state.incomingCall = null;
      if (!state.tableVoice) stopMic();
      await syncAllVoice();
    },
  };
}

export async function fetchInfo(base) {
  const url = new URL("/api/info", base || location.href);
  const res = await fetch(url, { cache: "no-store" });
  if (!res.ok) throw new Error("info");
  return res.json();
}

export async function postUpload(origin, rec, bytes) {
  const url = new URL("/api/upload", origin || location.href);
  url.searchParams.set("name", rec.name);
  url.searchParams.set("category", rec.category);
  url.searchParams.set("id", rec.id);
  url.searchParams.set("icon", rec.icon || "spark");
  url.searchParams.set("mood", rec.mood || "calm");
  const res = await fetch(url, {
    method: "POST",
    headers: { "Content-Type": rec.mime || "application/octet-stream" },
    body: bytes,
  });
  if (!res.ok) throw new Error("upload");
  return res.json();
}
