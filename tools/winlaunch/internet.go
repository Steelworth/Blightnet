package main

import (
	"bufio"
	"fmt"
	"io"
	"net/http"
	"os"
	"os/exec"
	"regexp"
	"strings"
	"sync"
	"time"
)

var netInfo = struct {
	mu    sync.Mutex
	wan   string
	relay string
	upnp  bool
}{}

var relayCmd *exec.Cmd
var relayMu sync.Mutex

func netSnapshot() (wan, relay string, upnp bool) {
	netInfo.mu.Lock()
	defer netInfo.mu.Unlock()
	return netInfo.wan, netInfo.relay, netInfo.upnp
}

func setWan(ip string) {
	netInfo.mu.Lock()
	netInfo.wan = ip
	netInfo.mu.Unlock()
}

func setRelay(url string) {
	netInfo.mu.Lock()
	netInfo.relay = strings.TrimRight(url, "/")
	netInfo.mu.Unlock()
}

func httpWANIP() string {
	client := &http.Client{Timeout: 3 * time.Second}
	for _, url := range []string{"https://api.ipify.org", "https://icanhazip.com", "https://ifconfig.me/ip"} {
		resp, err := client.Get(url)
		if err != nil {
			continue
		}
		b, _ := io.ReadAll(io.LimitReader(resp.Body, 64))
		resp.Body.Close()
		ip := strings.TrimSpace(string(b))
		parts := strings.Split(ip, ".")
		if len(parts) != 4 {
			continue
		}
		ok := true
		for _, p := range parts {
			if p == "" {
				ok = false
				break
			}
		}
		if ok {
			return ip
		}
	}
	return ""
}

func isCGNAT(ip string) bool {
	parts := strings.Split(ip, ".")
	if len(parts) != 4 {
		return false
	}
	var a, b int
	if _, err := fmt.Sscanf(ip, "%d.%d", &a, &b); err != nil {
		return false
	}
	return a == 100 && b >= 64 && b <= 127
}

var relayURL = regexp.MustCompile(`https?://[a-zA-Z0-9][a-zA-Z0-9.-]+\.[a-zA-Z]{2,}`)

func pickRelayURL(text string) string {
	for _, m := range relayURL.FindAllString(text, -1) {
		m = strings.TrimRight(m, "/.,)")
		if strings.HasPrefix(m, "http://") {
			m = "https://" + strings.TrimPrefix(m, "http://")
		}
		host := m
		if i := strings.Index(host, "://"); i >= 0 {
			host = host[i+3:]
		}
		if slash := strings.Index(host, "/"); slash >= 0 {
			host = host[:slash]
		}
		host = strings.ToLower(host)
		if host == "" || host == "github.com" || host == "localhost" {
			continue
		}
		if strings.HasSuffix(host, ".github.com") || strings.HasSuffix(host, ".google.com") {
			continue
		}
		return m
	}
	return ""
}

func sshBins() []string {
	out := []string{}
	seen := map[string]bool{}
	add := func(p string) {
		if p == "" || seen[p] {
			return
		}
		if st, err := os.Stat(p); err == nil && !st.IsDir() {
			seen[p] = true
			out = append(out, p)
		}
	}
	if p, err := exec.LookPath("ssh"); err == nil {
		add(p)
	}
	for _, p := range []string{
		`C:\Windows\System32\OpenSSH\ssh.exe`,
		`C:\Program Files\Git\usr\bin\ssh.exe`,
		`C:\Program Files\Git\bin\ssh.exe`,
		"/usr/bin/ssh",
		"/usr/local/bin/ssh",
	} {
		add(p)
	}
	return out
}

func startRelay(port int) {
	if _, relay, _ := netSnapshot(); relay != "" {
		return
	}
	if startNativeSSH(port) {
		return
	}
	local := fmt.Sprintf("127.0.0.1:%d", port)
	attempts := [][]string{}
	if path, err := exec.LookPath("cloudflared"); err == nil {
		attempts = append(attempts, []string{path, "tunnel", "--no-autoupdate", "--url", "http://" + local})
	}
	for _, path := range sshBins() {
		base := []string{
			path,
			"-T",
			"-o", "StrictHostKeyChecking=accept-new",
			"-o", "ServerAliveInterval=30",
			"-o", "ExitOnForwardFailure=yes",
		}
		attempts = append(attempts, append(append([]string{}, base...), "-p", "443", "-R", "0:"+local, "free@a.pinggy.io"))
		attempts = append(attempts, append(append([]string{}, base...), "-p", "443", "-R", "0:"+local, "a.pinggy.io"))
		attempts = append(attempts, append(append([]string{}, base...), "-R", "80:"+local, "nokey@localhost.run"))
		attempts = append(attempts, append(append([]string{}, base...), "-R", "80:"+local, "serveo.net"))
	}
	for _, cmd := range attempts {
		fmt.Println("Internet table: opening a path friends can reach…")
		if spawnRelay(cmd) {
			return
		}
	}
	if _, relay, _ := netSnapshot(); relay == "" {
		fmt.Println("Internet table: no automatic tunnel. LAN still works.")
	}
}

func spawnRelay(args []string) bool {
	cmd := exec.Command(args[0], args[1:]...)
	pr, pw := io.Pipe()
	cmd.Stdout = pw
	cmd.Stderr = pw
	cmd.Stdin = strings.NewReader("\n")
	if err := cmd.Start(); err != nil {
		_ = pw.Close()
		return false
	}
	go func() {
		_ = cmd.Wait()
		_ = pw.Close()
	}()
	stdout := pr
	relayMu.Lock()
	if relayCmd != nil && relayCmd.Process != nil {
		_ = relayCmd.Process.Kill()
	}
	relayCmd = cmd
	relayMu.Unlock()

	done := make(chan string, 1)
	go func() {
		sc := bufio.NewScanner(stdout)
		buf := ""
		for sc.Scan() {
			buf += sc.Text() + "\n"
			if u := pickRelayURL(buf); u != "" {
				select {
				case done <- u:
				default:
				}
			}
		}
	}()
	select {
	case url := <-done:
		setRelay(url)
		fmt.Println("Internet table (relay) →", url)
		return true
	case <-time.After(18 * time.Second):
		if _, relay, _ := netSnapshot(); relay != "" {
			return true
		}
		if cmd.Process != nil {
			_ = cmd.Process.Kill()
		}
		return false
	}
}

func punchInternet(port int) {
	if upnpMap(port) {
		setUPnP(true)
		fmt.Println("Internet table: router opened port", port)
	}
	wan := httpWANIP()
	if wan != "" && isCGNAT(wan) {
		fmt.Printf("Public IPv4 %s is carrier NAT — friends cannot dial it directly.\n", wan)
		wan = ""
		setUPnP(false)
	}
	if wan != "" {
		setWan(wan)
		mapped := ""
		if _, _, up := netSnapshot(); up {
			mapped = " (router opened)"
		}
		fmt.Printf("Internet table (IPv4) → http://%s:%d/%s\n", wan, port, mapped)
	}
	startRelay(port)
	if _, relay, upnp := netSnapshot(); relay == "" && !(upnp && wan != "") {
		fmt.Println("Internet table: no public address yet. Join still works on the LAN.")
	}
}
