package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"sync"
)

func foldName(value string) string {
	return strings.Join(strings.Fields(strings.ToLower(strings.TrimSpace(value))), " ")
}

type peerInfo struct {
	ID   string `json:"id"`
	Name string `json:"name"`
	Role string `json:"role"`
}

type fileRec struct {
	ID       string `json:"id"`
	Name     string `json:"name"`
	Category string `json:"category"`
	Mime     string `json:"mime"`
	Size     int    `json:"size"`
	Icon     string `json:"icon"`
	Mood     string `json:"mood"`
	File     string `json:"file"`
}

type Hub struct {
	mu       sync.Mutex
	clients  map[string]*wsClient
	lastMix   any
	lastMap   any
	lastChars map[string]map[string]any
	lastRolls []map[string]any
	lastPics  map[string]string
	root      string
	uploadDir string
}

func newHub(root string) *Hub {
	dir := filepath.Join(root, "uploads")
	_ = os.MkdirAll(dir, 0o755)
	return &Hub{clients: map[string]*wsClient{}, lastChars: map[string]map[string]any{}, lastRolls: []map[string]any{}, lastPics: map[string]string{}, root: root, uploadDir: dir}
}

func (h *Hub) add(c *wsClient) {
	h.mu.Lock()
	h.clients[c.id] = c
	h.mu.Unlock()
	h.broadcastPeers()
}

func (h *Hub) remove(c *wsClient) {
	h.mu.Lock()
	delete(h.clients, c.id)
	delete(h.lastChars, c.id)
	delete(h.lastPics, c.id)
	if c.role == "host" {
		h.lastMix = nil
		h.lastMap = nil
	}
	h.mu.Unlock()
	h.broadcastPeers()
	h.broadcast(map[string]any{"type": "chars", "from": c.id, "name": "", "list": []any{}}, "")
}

func (h *Hub) get(id string) *wsClient {
	h.mu.Lock()
	defer h.mu.Unlock()
	return h.clients[id]
}

func (h *Hub) namedExcept(except string) []*wsClient {
	h.mu.Lock()
	defer h.mu.Unlock()
	out := make([]*wsClient, 0)
	for _, c := range h.clients {
		if c.named && c.id != except {
			out = append(out, c)
		}
	}
	return out
}

func (h *Hub) findNamed(needle, except string) (*wsClient, string) {
	want := foldName(needle)
	if want == "" {
		return nil, "missing"
	}
	named := h.namedExcept(except)
	var exact []*wsClient
	for _, c := range named {
		if foldName(c.name) == want {
			exact = append(exact, c)
		}
	}
	if len(exact) == 1 {
		return exact[0], ""
	}
	if len(exact) > 1 {
		return nil, "ambiguous"
	}
	var prefixes []*wsClient
	for _, c := range named {
		if strings.HasPrefix(foldName(c.name), want) {
			prefixes = append(prefixes, c)
		}
	}
	if len(prefixes) == 1 {
		return prefixes[0], ""
	}
	if len(prefixes) > 1 {
		return nil, "ambiguous"
	}
	return nil, "missing"
}

func (h *Hub) rememberRoll(row map[string]any) {
	h.mu.Lock()
	h.lastRolls = append(h.lastRolls, row)
	if len(h.lastRolls) > 40 {
		h.lastRolls = h.lastRolls[len(h.lastRolls)-40:]
	}
	h.mu.Unlock()
}

func (h *Hub) picsTable() []map[string]any {
	h.mu.Lock()
	defer h.mu.Unlock()
	out := make([]map[string]any, 0, len(h.lastPics))
	for pid, pic := range h.lastPics {
		if pic != "" {
			out = append(out, map[string]any{"from": pid, "pic": pic})
		}
	}
	return out
}

func (h *Hub) rolls() []map[string]any {
	h.mu.Lock()
	defer h.mu.Unlock()
	out := make([]map[string]any, len(h.lastRolls))
	copy(out, h.lastRolls)
	return out
}

func (h *Hub) sendJSON(c *wsClient, msg any) {
	if c == nil {
		return
	}
	body, err := json.Marshal(msg)
	if err != nil {
		return
	}
	c.send(body)
}

func (h *Hub) host() *wsClient {
	h.mu.Lock()
	defer h.mu.Unlock()
	for _, c := range h.clients {
		if c.role == "host" && c.named {
			return c
		}
	}
	return nil
}

func (h *Hub) hasHostExcept(except *wsClient) bool {
	h.mu.Lock()
	defer h.mu.Unlock()
	for _, c := range h.clients {
		if c != except && c.role == "host" && c.named {
			return true
		}
	}
	return false
}

func (h *Hub) peers() []peerInfo {
	h.mu.Lock()
	defer h.mu.Unlock()
	out := make([]peerInfo, 0, len(h.clients))
	for _, c := range h.clients {
		if c.named {
			out = append(out, peerInfo{ID: c.id, Name: c.name, Role: c.role})
		}
	}
	return out
}

func (h *Hub) broadcast(msg any, exclude string) {
	body, err := json.Marshal(msg)
	if err != nil {
		return
	}
	h.mu.Lock()
	list := make([]*wsClient, 0, len(h.clients))
	for _, c := range h.clients {
		if exclude != "" && c.id == exclude {
			continue
		}
		list = append(list, c)
	}
	h.mu.Unlock()
	for _, c := range list {
		c.send(body)
	}
}

func (h *Hub) broadcastPeers() {
	h.broadcast(map[string]any{"type": "peers", "peers": h.peers()}, "")
}

func (h *Hub) charsTable() []map[string]any {
	h.mu.Lock()
	defer h.mu.Unlock()
	out := make([]map[string]any, 0, len(h.lastChars))
	for pid, pack := range h.lastChars {
		out = append(out, map[string]any{
			"from": pid,
			"name": pack["name"],
			"list": pack["list"],
		})
	}
	return out
}

func (h *Hub) loadManifest() []fileRec {
	raw, err := os.ReadFile(filepath.Join(h.uploadDir, "manifest.json"))
	if err != nil {
		return []fileRec{}
	}
	var rows []fileRec
	if json.Unmarshal(raw, &rows) != nil {
		return []fileRec{}
	}
	return rows
}

func (h *Hub) saveManifest(rows []fileRec) error {
	raw, err := json.Marshal(rows)
	if err != nil {
		return err
	}
	return os.WriteFile(filepath.Join(h.uploadDir, "manifest.json"), raw, 0o644)
}
