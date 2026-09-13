package main

import (
	"crypto/sha1"
	"encoding/base64"
	"encoding/binary"
	"encoding/json"
	"io"
	"net/http"
	"regexp"
	"strings"
	"sync"
	"time"
	"unicode/utf8"
)

var whisperCmd = regexp.MustCompile(`(?i)^/(?:w|whisper|msg|tell)(?:\s+|$)(.*)`)

const wsGUID = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11"
const maxWS = 4 * 1024 * 1024

type wsClient struct {
	id    string
	name  string
	role  string
	named bool
	pic   string
	w     http.ResponseWriter
	mu    sync.Mutex
	alive bool
	hj    http.Hijacker
	rw    interface {
		io.ReadWriter
		Flush() error
	}
}

func (c *wsClient) send(payload []byte) {
	if !c.alive {
		return
	}
	header := []byte{0x81}
	n := len(payload)
	if n < 126 {
		header = append(header, byte(n))
	} else if n < 65536 {
		header = append(header, 126, 0, 0)
		binary.BigEndian.PutUint16(header[2:], uint16(n))
	} else {
		header = append(header, 127, 0, 0, 0, 0, 0, 0, 0, 0)
		binary.BigEndian.PutUint64(header[2:], uint64(n))
	}
	c.mu.Lock()
	defer c.mu.Unlock()
	if !c.alive {
		return
	}
	_, err := c.rw.Write(append(header, payload...))
	if err == nil {
		err = c.rw.Flush()
	}
	if err != nil {
		c.alive = false
	}
}

func wsAccept(key string) string {
	sum := sha1.Sum([]byte(key + wsGUID))
	return base64.StdEncoding.EncodeToString(sum[:])
}

func (h *Hub) serveWS(w http.ResponseWriter, r *http.Request) {
	if !strings.Contains(strings.ToLower(r.Header.Get("Connection")), "upgrade") {
		http.Error(w, "upgrade required", http.StatusBadRequest)
		return
	}
	key := r.Header.Get("Sec-WebSocket-Key")
	if key == "" {
		http.Error(w, "missing key", http.StatusBadRequest)
		return
	}
	hj, ok := w.(http.Hijacker)
	if !ok {
		http.Error(w, "no hijack", http.StatusInternalServerError)
		return
	}
	conn, bufrw, err := hj.Hijack()
	if err != nil {
		return
	}
	resp := "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: " + wsAccept(key) + "\r\n\r\n"
	if _, err := bufrw.WriteString(resp); err != nil {
		conn.Close()
		return
	}
	_ = bufrw.Flush()
	client := &wsClient{id: randID(), role: "guest", alive: true, w: w, rw: bufrw}
	defer func() {
		client.alive = false
		h.remove(client)
		conn.Close()
	}()
	for client.alive {
		_ = conn.SetReadDeadline(time.Now().Add(120 * time.Second))
		msg, err := readWS(bufrw)
		if err != nil {
			return
		}
		if msg == "" {
			continue
		}
		var data map[string]any
		if json.Unmarshal([]byte(msg), &data) != nil {
			continue
		}
		h.handle(client, data)
	}
}

func splitWhisper(text string) (bool, string) {
	m := whisperCmd.FindStringSubmatch(text)
	if m == nil {
		return false, text
	}
	return true, strings.TrimSpace(m[1])
}

func whisperError(kind, hint string) string {
	who := strings.TrimSpace(hint)
	if who == "" {
		who = "that name"
	}
	switch kind {
	case "self":
		return "Whisper another player, not yourself."
	case "ambiguous":
		return "Several players match " + who + ". Use the full handle."
	case "usage":
		return "Whisper with /w Name then your message, or click a name above."
	case "empty":
		return "Type a whisper after the name."
	default:
		return "No one named " + who + " is at the table."
	}
}

func stripNamePrefix(body, name string) string {
	body = strings.TrimSpace(body)
	if body == "" {
		return body
	}
	if strings.HasPrefix(body, "\"") || strings.HasPrefix(body, "'") {
		q := body[:1]
		end := strings.Index(body[1:], q)
		if end >= 0 {
			inner := strings.TrimSpace(body[1 : 1+end])
			if foldName(inner) == foldName(name) {
				return strings.TrimSpace(body[2+end:])
			}
		}
	}
	n := strings.TrimSpace(name)
	if n == "" {
		return body
	}
	if len(body) >= len(n) && strings.EqualFold(body[:len(n)], n) {
		if len(body) == len(n) || body[len(n)] == ' ' {
			return strings.TrimSpace(body[len(n):])
		}
	}
	return body
}

func (h *Hub) resolveWhisperRest(rest, selfID string) (*wsClient, string, string, string) {
	rest = strings.TrimSpace(rest)
	if rest == "" {
		return nil, "", "usage", ""
	}
	if strings.HasPrefix(rest, "\"") || strings.HasPrefix(rest, "'") {
		q := rest[:1]
		end := strings.Index(rest[1:], q)
		if end < 0 {
			return nil, "", "usage", ""
		}
		name := strings.TrimSpace(rest[1 : 1+end])
		body := strings.TrimSpace(rest[2+end:])
		target, err := h.findNamed(name, selfID)
		return target, body, err, name
	}
	peers := h.namedExcept(selfID)
	restL := strings.ToLower(rest)
	var best *wsClient
	bestLen := -1
	for _, p := range peers {
		name := strings.TrimSpace(p.name)
		if name == "" {
			continue
		}
		low := strings.ToLower(name)
		if restL == low || strings.HasPrefix(restL, low+" ") {
			if len(name) > bestLen {
				best = p
				bestLen = len(name)
			}
		}
	}
	if best != nil {
		return best, strings.TrimSpace(rest[bestLen:]), "", best.name
	}
	token, leftover, _ := strings.Cut(rest, " ")
	target, err := h.findNamed(token, selfID)
	if target != nil {
		return target, strings.TrimSpace(leftover), "", target.name
	}
	if err == "" {
		err = "missing"
	}
	return nil, strings.TrimSpace(leftover), err, token
}

func (h *Hub) deliverWhisper(sender *wsClient, data map[string]any, text string) {
	toID, _ := data["to"].(string)
	toID = strings.TrimSpace(toID)
	isCmd, rest := splitWhisper(text)
	var target *wsClient
	errKind := ""
	hint := ""
	body := text
	if toID != "" {
		other := h.get(toID)
		if other == sender || (other != nil && other.id == sender.id) {
			errKind = "self"
			hint = sender.name
		} else if other == nil || !other.named {
			errKind = "missing"
			hint = toID
		} else {
			target = other
			if isCmd {
				body = stripNamePrefix(rest, other.name)
			} else {
				body = text
			}
		}
	}
	if target == nil && errKind == "" {
		if isCmd {
			target, body, errKind, hint = h.resolveWhisperRest(rest, sender.id)
		} else {
			errKind = "missing"
			hint = toID
		}
	}
	now := time.Now().UnixMilli()
	if errKind != "" || target == nil {
		h.sendJSON(sender, map[string]any{
			"type": "chat",
			"sys":  true,
			"text": whisperError(errKind, hint),
			"ts":   now,
		})
		return
	}
	body = strings.TrimSpace(body)
	if utf8.RuneCountInString(body) > 400 {
		body = string([]rune(body)[:400])
	}
	if body == "" {
		h.sendJSON(sender, map[string]any{
			"type": "chat",
			"sys":  true,
			"text": whisperError("empty", ""),
			"ts":   now,
		})
		return
	}
	payload := map[string]any{
		"type":    "chat",
		"id":      sender.id,
		"name":    sender.name,
		"text":    body,
		"ts":      now,
		"whisper": true,
		"to":      target.id,
		"toName":  target.name,
	}
	raw, err := json.Marshal(payload)
	if err != nil {
		return
	}
	sender.send(raw)
	if target != sender {
		target.send(raw)
	}
}

func (h *Hub) handle(c *wsClient, data map[string]any) {
	kind, _ := data["type"].(string)
	if kind == "hello" {
		name, _ := data["name"].(string)
		name = strings.TrimSpace(name)
		if name == "" {
			name = "Traveller"
		}
		if utf8.RuneCountInString(name) > 24 {
			name = string([]rune(name)[:24])
		}
		role, _ := data["role"].(string)
		if role != "host" {
			role = "guest"
		}
		if role == "host" && h.hasHostExcept(c) {
			role = "guest"
		}
		if want, _ := data["id"].(string); want != "" && h.get(want) == nil {
			c.id = want
		}
		c.name = name
		c.role = role
		c.named = true
		pic, _ := data["pic"].(string)
		if !strings.HasPrefix(pic, "data:image/") || !strings.Contains(pic, "base64,") || len(pic) > 120000 {
			pic = ""
		}
		c.pic = pic
		h.add(c)
		h.mu.Lock()
		if pic != "" {
			h.lastPics[c.id] = pic
		} else {
			delete(h.lastPics, c.id)
		}
		h.mu.Unlock()
		welcome, _ := json.Marshal(map[string]any{
			"type":  "welcome",
			"id":    c.id,
			"role":  c.role,
			"peers": h.peers(),
			"files": h.loadManifest(),
			"mix":   h.lastMix,
			"map":   h.lastMap,
			"chars": h.charsTable(),
			"rolls": h.rolls(),
			"pics":  h.picsTable(),
		})
		c.send(welcome)
		h.broadcastPeers()
		if pic != "" {
			h.broadcast(map[string]any{"type": "profile", "from": c.id, "name": c.name, "pic": pic}, c.id)
		}
		return
	}
	if !c.named {
		return
	}
	switch kind {
	case "chat":
		text, _ := data["text"].(string)
		text = strings.TrimSpace(text)
		if text == "" {
			return
		}
		if utf8.RuneCountInString(text) > 400 {
			text = string([]rune(text)[:400])
		}
		toID, _ := data["to"].(string)
		toID = strings.TrimSpace(toID)
		whisperFlag, _ := data["whisper"].(bool)
		isCmd, _ := splitWhisper(text)
		if toID != "" || isCmd || whisperFlag {
			h.deliverWhisper(c, data, text)
			return
		}
		h.broadcast(map[string]any{
			"type": "chat",
			"id":   c.id,
			"name": c.name,
			"text": text,
			"ts":   time.Now().UnixMilli(),
		}, "")
	case "mix":
		if c.role != "host" {
			return
		}
		mix := data["mix"]
		if m, ok := mix.(map[string]any); ok {
			delete(m, "master")
			mix = m
		}
		h.mu.Lock()
		h.lastMix = mix
		h.mu.Unlock()
		h.broadcast(map[string]any{"type": "mix", "mix": mix, "from": c.id}, c.id)
	case "roll":
		who, _ := data["who"].(string)
		who = strings.TrimSpace(who)
		if who == "" {
			who = c.name
		}
		if utf8.RuneCountInString(who) > 48 {
			who = string([]rune(who)[:48])
		}
		action, _ := data["action"].(string)
		action = strings.TrimSpace(action)
		if action == "" {
			action = "roll"
		}
		if utf8.RuneCountInString(action) > 80 {
			action = string([]rune(action)[:80])
		}
		pctF, ok := data["pct"].(float64)
		if !ok {
			return
		}
		pct := int(pctF)
		if pct < 0 {
			pct = 0
		}
		if pct > 100 {
			pct = 100
		}
		grade, _ := data["grade"].(string)
		if utf8.RuneCountInString(grade) > 24 {
			grade = string([]rune(grade)[:24])
		}
		var dmg any
		if data["damage"] != nil {
			if df, ok := data["damage"].(float64); ok {
				dmg = int(df)
			}
		}
		player, _ := data["player"].(string)
		player = strings.TrimSpace(player)
		if player == "" {
			player = c.name
		}
		if utf8.RuneCountInString(player) > 24 {
			player = string([]rune(player)[:24])
		}
		hack, _ := data["hack"].(bool)
		taken, _ := data["taken"].(bool)
		heal, _ := data["heal"].(bool)
		deathSave, _ := data["deathSave"].(bool)
		targetId, _ := data["targetId"].(string)
		targetName, _ := data["targetName"].(string)
		targetOwner, _ := data["targetOwner"].(string)
		tokenId, _ := data["tokenId"].(string)
		sheetId, _ := data["sheetId"].(string)
		sheetOwner, _ := data["sheetOwner"].(string)
		adv, _ := data["adv"].(string)
		adv = strings.TrimSpace(adv)
		if adv != "adv" && adv != "dis" {
			adv = ""
		}
		var pctA any
		var pctB any
		if f, ok := data["pctA"].(float64); ok {
			n := int(f)
			if n < 0 {
				n = 0
			}
			if n > 100 {
				n = 100
			}
			pctA = n
		}
		if f, ok := data["pctB"].(float64); ok {
			n := int(f)
			if n < 0 {
				n = 0
			}
			if n > 100 {
				n = 100
			}
			pctB = n
		}
		if utf8.RuneCountInString(targetId) > 64 {
			targetId = string([]rune(targetId)[:64])
		}
		if utf8.RuneCountInString(targetName) > 48 {
			targetName = string([]rune(targetName)[:48])
		}
		if utf8.RuneCountInString(targetOwner) > 24 {
			targetOwner = string([]rune(targetOwner)[:24])
		}
		if utf8.RuneCountInString(tokenId) > 32 {
			tokenId = string([]rune(tokenId)[:32])
		}
		if utf8.RuneCountInString(sheetId) > 64 {
			sheetId = string([]rune(sheetId)[:64])
		}
		if utf8.RuneCountInString(sheetOwner) > 24 {
			sheetOwner = string([]rune(sheetOwner)[:24])
		}
		row := map[string]any{
			"type":        "roll",
			"from":        c.id,
			"player":      player,
			"who":         who,
			"action":      action,
			"pct":         pct,
			"grade":       grade,
			"damage":      dmg,
			"taken":       taken,
			"heal":        heal,
			"hack":        hack,
			"deathSave":   deathSave,
			"targetId":    targetId,
			"targetName":  targetName,
			"targetOwner": targetOwner,
			"tokenId":     tokenId,
			"sheetId":     sheetId,
			"sheetOwner":  sheetOwner,
			"adv":         adv,
			"pctA":        pctA,
			"pctB":        pctB,
			"ts":          time.Now().UnixMilli(),
		}
		h.rememberRoll(row)
		h.broadcast(row, c.id)
	case "log":
		who, _ := data["who"].(string)
		who = strings.TrimSpace(who)
		if who == "" {
			who = c.name
		}
		if utf8.RuneCountInString(who) > 48 {
			who = string([]rune(who)[:48])
		}
		action, _ := data["action"].(string)
		action = strings.TrimSpace(action)
		if action == "" {
			action = "look"
		}
		if utf8.RuneCountInString(action) > 80 {
			action = string([]rune(action)[:80])
		}
		text, _ := data["text"].(string)
		if utf8.RuneCountInString(text) > 800 {
			text = string([]rune(text)[:800])
		}
		player, _ := data["player"].(string)
		player = strings.TrimSpace(player)
		if player == "" {
			player = c.name
		}
		note := map[string]any{
			"type":   "log",
			"info":   true,
			"from":   c.id,
			"player": player,
			"who":    who,
			"action": action,
			"text":   text,
			"ts":     time.Now().UnixMilli(),
		}
		h.rememberRoll(note)
		h.broadcast(note, c.id)
	case "map":
		if c.role != "host" {
			return
		}
		h.mu.Lock()
		h.lastMap = data["map"]
		h.mu.Unlock()
		h.broadcast(map[string]any{"type": "map", "map": data["map"], "from": c.id}, c.id)
	case "map-edit":
		host := h.host()
		if host == nil || host.id == c.id {
			return
		}
		body, err := json.Marshal(map[string]any{
			"type": "map-edit",
			"from": c.id,
			"edit": data["edit"],
		})
		if err == nil {
			host.send(body)
		}
	case "signal":
		to, _ := data["to"].(string)
		other := h.get(to)
		if other == nil {
			return
		}
		body, _ := json.Marshal(map[string]any{
			"type":    "signal",
			"from":    c.id,
			"to":      to,
			"payload": data["payload"],
		})
		other.send(body)
	case "voice":
		action, _ := data["action"].(string)
		if action != "invite" && action != "accept" && action != "decline" && action != "hangup" {
			return
		}
		to, _ := data["to"].(string)
		to = strings.TrimSpace(to)
		other := h.get(to)
		if other == nil || !other.named {
			return
		}
		body, _ := json.Marshal(map[string]any{
			"type":   "voice",
			"action": action,
			"from":   c.id,
			"name":   c.name,
			"to":     to,
		})
		other.send(body)
	case "profile":
		pic, _ := data["pic"].(string)
		if !strings.HasPrefix(pic, "data:image/") || !strings.Contains(pic, "base64,") || len(pic) > 120000 {
			pic = ""
		}
		c.pic = pic
		h.mu.Lock()
		if pic != "" {
			h.lastPics[c.id] = pic
		} else {
			delete(h.lastPics, c.id)
		}
		h.mu.Unlock()
		h.broadcast(map[string]any{"type": "profile", "from": c.id, "name": c.name, "pic": pic}, c.id)
	case "chars":
		rows, _ := data["list"].([]any)
		if rows == nil {
			rows = []any{}
		}
		if len(rows) > 16 {
			rows = rows[:16]
		}
		h.mu.Lock()
		h.lastChars[c.id] = map[string]any{"name": c.name, "list": rows}
		h.mu.Unlock()
		h.broadcast(map[string]any{"type": "chars", "from": c.id, "name": c.name, "list": rows}, c.id)
	case "files":
		if c.role == "host" {
			h.broadcast(map[string]any{"type": "files", "files": data["files"]}, c.id)
		}
	}
}

func readWS(r io.Reader) (string, error) {
	head := make([]byte, 2)
	if _, err := io.ReadFull(r, head); err != nil {
		return "", err
	}
	opcode := head[0] & 0x0f
	masked := head[1]&0x80 != 0
	n := int(head[1] & 0x7f)
	if n == 126 {
		ext := make([]byte, 2)
		if _, err := io.ReadFull(r, ext); err != nil {
			return "", err
		}
		n = int(binary.BigEndian.Uint16(ext))
	} else if n == 127 {
		ext := make([]byte, 8)
		if _, err := io.ReadFull(r, ext); err != nil {
			return "", err
		}
		n = int(binary.BigEndian.Uint64(ext))
	}
	if n > maxWS {
		return "", io.ErrUnexpectedEOF
	}
	var mask []byte
	if masked {
		mask = make([]byte, 4)
		if _, err := io.ReadFull(r, mask); err != nil {
			return "", err
		}
	}
	payload := make([]byte, n)
	if n > 0 {
		if _, err := io.ReadFull(r, payload); err != nil {
			return "", err
		}
	}
	if masked {
		for i := range payload {
			payload[i] ^= mask[i%4]
		}
	}
	if opcode == 0x8 {
		return "", io.EOF
	}
	if opcode == 0x9 {
		return "", nil
	}
	if opcode == 0x1 || opcode == 0x2 {
		return string(payload), nil
	}
	return "", nil
}
