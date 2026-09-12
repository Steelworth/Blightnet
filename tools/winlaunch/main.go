// Blightnet Windows launcher: local static server, then open the browser.
package main

import (
	"crypto/rand"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"io"
	"net"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"strings"
	"time"
)

const defaultPort = 8765
const maxUpload = 40 * 1024 * 1024

func main() {
	root := appRoot()
	index := filepath.Join(root, "index.html")
	if _, err := os.Stat(index); err != nil {
		fatal("Could not find index.html next to Blightnet.exe.\nPut Blightnet.exe in the blightnet folder (the one with index.html, audio, and assets).")
	}

	port := pickPort(defaultPort)
	hub := newHub(root)
	initUpdate(root)
	srv := &http.Server{
		Addr:              fmt.Sprintf("0.0.0.0:%d", port),
		Handler:           withCORS(newMux(root, port, hub)),
		ReadHeaderTimeout: 8 * time.Second,
	}

	ln, err := net.Listen("tcp", srv.Addr)
	if err != nil {
		fatal("Could not listen on %s: %v", srv.Addr, err)
	}

	url := fmt.Sprintf("http://127.0.0.1:%d/", port)
	fmt.Printf("Blightnet → %s\n", url)
	for _, ip := range lanIPs() {
		fmt.Printf("  table  → http://%s:%d/\n", ip, port)
	}
	fmt.Println("Leave this window open while you play. Close it to stop.")
	go openBrowser(url)
	if err := srv.Serve(ln); err != nil && err != http.ErrServerClosed {
		fatal("Server stopped: %v", err)
	}
}

func newMux(root string, port int, hub *Hub) http.Handler {
	mux := http.NewServeMux()
	files := &fileHandler{root: root}
	mux.HandleFunc("/ws", hub.serveWS)
	mux.HandleFunc("/api/info", func(w http.ResponseWriter, r *http.Request) {
		writeJSON(w, map[string]any{
			"ok":   true,
			"app":  "blightnet",
			"port": port,
			"ips":  lanIPs(),
			"url":  fmt.Sprintf("http://127.0.0.1:%d/", port),
		})
	})
	mux.HandleFunc("/__hearthsong", func(w http.ResponseWriter, r *http.Request) {
		writeJSON(w, map[string]any{"ok": true, "app": "blightnet"})
	})
	mux.HandleFunc("/__blightnet", func(w http.ResponseWriter, r *http.Request) {
		writeJSON(w, map[string]any{"ok": true, "app": "blightnet"})
	})
	mux.HandleFunc("/api/uploads", func(w http.ResponseWriter, r *http.Request) {
		writeJSON(w, map[string]any{"files": hub.loadManifest()})
	})
	mux.HandleFunc("/api/update", func(w http.ResponseWriter, r *http.Request) {
		if r.Method == http.MethodPost {
			writeJSON(w, startUpdate(root))
			return
		}
		writeJSON(w, updateSnapshot())
	})
	mux.HandleFunc("/api/upload", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			http.Error(w, "method", http.StatusMethodNotAllowed)
			return
		}
		handleUpload(w, r, hub)
	})
	mux.Handle("/uploads/", http.StripPrefix("/uploads/", http.FileServer(http.Dir(hub.uploadDir))))
	mux.Handle("/", files)
	return mux
}

func handleUpload(w http.ResponseWriter, r *http.Request, hub *Hub) {
	if r.ContentLength <= 0 || r.ContentLength > maxUpload {
		writeJSONStatus(w, 400, map[string]any{"ok": false, "error": "bad size"})
		return
	}
	q := r.URL.Query()
	name := first(q.Get("name"), r.Header.Get("X-Hearth-Name"), "Upload")
	if len(name) > 80 {
		name = name[:80]
	}
	category := first(q.Get("category"), r.Header.Get("X-Hearth-Category"), "music")
	if category != "music" && category != "weather" && category != "animals" && category != "ambience" && category != "map" {
		category = "music"
	}
	id := sanitizeID(first(q.Get("id"), r.Header.Get("X-Hearth-Id"), ""))
	if id == "" {
		id = "up-" + randID()
	}
	icon := first(q.Get("icon"), r.Header.Get("X-Hearth-Icon"), "spark")
	mood := first(q.Get("mood"), r.Header.Get("X-Hearth-Mood"), "calm")
	mime := r.Header.Get("Content-Type")
	if mime == "" {
		mime = "application/octet-stream"
	}
	mime = strings.Split(mime, ";")[0]
	ext := map[string]string{
		"audio/ogg": ".ogg", "audio/mpeg": ".mp3", "audio/wav": ".wav",
		"audio/x-wav": ".wav", "audio/mp4": ".m4a", "audio/flac": ".flac", "audio/webm": ".webm",
		"image/jpeg": ".jpg", "image/jpg": ".jpg", "image/png": ".png", "image/webp": ".webp", "image/gif": ".gif",
	}[mime]
	if ext == "" {
		if n := strings.ToLower(filepath.Ext(name)); n != "" {
			ext = n
		}
	}
	if ext == "" {
		if category == "map" {
			ext = ".jpg"
		} else {
			ext = ".ogg"
		}
	}
	body, err := io.ReadAll(io.LimitReader(r.Body, maxUpload+1))
	if err != nil || len(body) == 0 || len(body) > maxUpload {
		writeJSONStatus(w, 400, map[string]any{"ok": false, "error": "bad body"})
		return
	}
	_ = os.MkdirAll(hub.uploadDir, 0o755)
	filename := id + ext
	if err := os.WriteFile(filepath.Join(hub.uploadDir, filename), body, 0o644); err != nil {
		writeJSONStatus(w, 500, map[string]any{"ok": false, "error": "write"})
		return
	}
	rec := fileRec{ID: id, Name: name, Category: category, Mime: mime, Size: len(body), Icon: icon, Mood: mood, File: "uploads/" + filename}
	rows := hub.loadManifest()
	out := []fileRec{}
	for _, row := range rows {
		if row.ID != id {
			out = append(out, row)
		}
	}
	out = append(out, rec)
	_ = hub.saveManifest(out)
	writeJSON(w, map[string]any{"ok": true, "file": rec})
}

func withCORS(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Access-Control-Allow-Origin", "*")
		w.Header().Set("Access-Control-Allow-Headers", "Content-Type, X-Hearth-Name, X-Hearth-Category, X-Hearth-Id, X-Hearth-Icon, X-Hearth-Mood")
		w.Header().Set("Access-Control-Allow-Methods", "GET, POST, OPTIONS, HEAD")
		if r.Method == http.MethodOptions {
			w.WriteHeader(http.StatusNoContent)
			return
		}
		next.ServeHTTP(w, r)
	})
}

type fileHandler struct{ root string }

func (h *fileHandler) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet && r.Method != http.MethodHead {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}
	path := strings.SplitN(r.URL.Path, "?", 2)[0]
	rel := strings.TrimPrefix(path, "/")
	if rel == "" || strings.HasSuffix(path, "/") {
		rel = "index.html"
	}
	rel = filepath.FromSlash(rel)
	if strings.Contains(rel, "..") {
		http.NotFound(w, r)
		return
	}
	full := filepath.Join(h.root, rel)
	root := h.root
	if !strings.HasSuffix(root, string(os.PathSeparator)) {
		root += string(os.PathSeparator)
	}
	if full != filepath.Clean(h.root) && !strings.HasPrefix(full, root) {
		http.NotFound(w, r)
		return
	}
	info, err := os.Stat(full)
	if err != nil || info.IsDir() {
		http.NotFound(w, r)
		return
	}
	if ctype := mimeType(full); ctype != "" {
		w.Header().Set("Content-Type", ctype)
	}
	if noStore(full) {
		w.Header().Set("Cache-Control", "no-store")
	}
	http.ServeFile(w, r, full)
}

func noStore(path string) bool {
	ext := strings.ToLower(filepath.Ext(path))
	return ext == ".html" || ext == ".js" || ext == ".mjs" || ext == ".css"
}

func mimeType(path string) string {
	switch strings.ToLower(filepath.Ext(path)) {
	case ".html":
		return "text/html; charset=utf-8"
	case ".js", ".mjs":
		return "text/javascript; charset=utf-8"
	case ".css":
		return "text/css; charset=utf-8"
	case ".json":
		return "application/json"
	case ".png":
		return "image/png"
	case ".jpg", ".jpeg":
		return "image/jpeg"
	case ".svg":
		return "image/svg+xml"
	case ".ogg", ".oga":
		return "audio/ogg"
	case ".wav":
		return "audio/wav"
	case ".mp3":
		return "audio/mpeg"
	case ".m4a":
		return "audio/mp4"
	case ".flac":
		return "audio/flac"
	case ".mp4":
		return "video/mp4"
	case ".webm":
		return "video/webm"
	case ".gif":
		return "image/gif"
	case ".webp":
		return "image/webp"
	case ".ico":
		return "image/x-icon"
	default:
		return ""
	}
}

func appRoot() string {
	exe, err := os.Executable()
	if err == nil {
		if resolved, err := filepath.EvalSymlinks(exe); err == nil {
			exe = resolved
		}
		return filepath.Dir(exe)
	}
	wd, err := os.Getwd()
	if err != nil {
		return "."
	}
	return wd
}

func pickPort(preferred int) int {
	l, err := net.Listen("tcp", fmt.Sprintf("0.0.0.0:%d", preferred))
	if err == nil {
		addr := l.Addr().(*net.TCPAddr)
		_ = l.Close()
		return addr.Port
	}
	l, err = net.Listen("tcp", "0.0.0.0:0")
	if err != nil {
		return preferred
	}
	addr := l.Addr().(*net.TCPAddr)
	_ = l.Close()
	fmt.Printf("Port %d is busy. Using %d instead.\n", preferred, addr.Port)
	return addr.Port
}

func lanIPs() []string {
	found := []string{}
	add := func(ip string) {
		if ip == "" || strings.HasPrefix(ip, "127.") {
			return
		}
		if net.ParseIP(ip) == nil || strings.Contains(ip, ":") {
			return
		}
		for _, x := range found {
			if x == ip {
				return
			}
		}
		found = append(found, ip)
	}
	ifaces, _ := net.Interfaces()
	for _, iface := range ifaces {
		if iface.Flags&net.FlagUp == 0 || iface.Flags&net.FlagLoopback != 0 {
			continue
		}
		addrs, _ := iface.Addrs()
		for _, a := range addrs {
			if ipnet, ok := a.(*net.IPNet); ok && ipnet.IP.To4() != nil {
				add(ipnet.IP.String())
			}
		}
	}
	return found
}

func openBrowser(u string) {
	time.Sleep(250 * time.Millisecond)
	app := [][]string{
		{"cmd", "/C", "start", "", "msedge", "--app=" + u},
		{"cmd", "/C", "start", "", "chrome", "--app=" + u},
	}
	for _, c := range app {
		cmd := exec.Command(c[0], c[1:]...)
		if err := cmd.Start(); err == nil {
			return
		}
	}
	_ = exec.Command("cmd", "/C", "start", "", u).Start()
}

func fatal(format string, args ...any) {
	fmt.Fprintf(os.Stderr, format+"\n", args...)
	fmt.Fprintln(os.Stderr, "Press Enter to close.")
	_, _ = fmt.Scanln()
	os.Exit(1)
}

func writeJSON(w http.ResponseWriter, obj any) {
	writeJSONStatus(w, 200, obj)
}

func writeJSONStatus(w http.ResponseWriter, code int, obj any) {
	w.Header().Set("Content-Type", "application/json")
	w.Header().Set("Cache-Control", "no-store")
	w.WriteHeader(code)
	_ = json.NewEncoder(w).Encode(obj)
}

func first(vals ...string) string {
	for _, v := range vals {
		if strings.TrimSpace(v) != "" {
			return strings.TrimSpace(v)
		}
	}
	return ""
}

var idRe = regexp.MustCompile(`[^a-zA-Z0-9_-]`)

func sanitizeID(s string) string {
	s = idRe.ReplaceAllString(s, "")
	if len(s) > 40 {
		s = s[:40]
	}
	return s
}

func randID() string {
	b := make([]byte, 6)
	_, _ = rand.Read(b)
	return hex.EncodeToString(b)
}
