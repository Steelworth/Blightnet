package main

import (
	"bufio"
	"fmt"
	"io"
	"net"
	"sync"
	"time"

	"golang.org/x/crypto/ssh"
)

var (
	sshMu     sync.Mutex
	sshClient *ssh.Client
)

type sshHop struct {
	addr   string
	user   string
	remote string
}

func closeSSH() {
	sshMu.Lock()
	c := sshClient
	sshClient = nil
	sshMu.Unlock()
	if c != nil {
		_ = c.Close()
	}
}

func proxyTunnel(a, b net.Conn) {
	defer a.Close()
	defer b.Close()
	go func() {
		_, _ = io.Copy(a, b)
		_ = a.Close()
		_ = b.Close()
	}()
	_, _ = io.Copy(b, a)
}

func sshConfig(user string) *ssh.ClientConfig {
	empty := func(user, instruction string, questions []string, echos []bool) ([]string, error) {
		return make([]string, len(questions)), nil
	}
	return &ssh.ClientConfig{
		User: user,
		Auth: []ssh.AuthMethod{
			ssh.Password(""),
			ssh.Password("0"),
			ssh.KeyboardInteractive(empty),
		},
		HostKeyCallback: ssh.InsecureIgnoreHostKey(),
		Timeout:         12 * time.Second,
		ClientVersion:   "SSH-2.0-Blightnet",
	}
}

func keepSSH(client *ssh.Client) {
	t := time.NewTicker(25 * time.Second)
	defer t.Stop()
	for range t.C {
		_, _, err := client.SendRequest("keepalive@openssh.com", true, nil)
		if err != nil {
			return
		}
	}
}

func tryNativeSSH(localPort int, hop sshHop) bool {
	client, err := ssh.Dial("tcp", hop.addr, sshConfig(hop.user))
	if err != nil {
		return false
	}
	ln, err := client.Listen("tcp", hop.remote)
	if err != nil {
		_ = client.Close()
		return false
	}
	go func() {
		for {
			remote, err := ln.Accept()
			if err != nil {
				return
			}
			local, err := net.DialTimeout("tcp", fmt.Sprintf("127.0.0.1:%d", localPort), 4*time.Second)
			if err != nil {
				_ = remote.Close()
				continue
			}
			go proxyTunnel(remote, local)
		}
	}()
	go keepSSH(client)

	got := make(chan string, 1)
	go func() {
		sess, err := client.NewSession()
		if err != nil {
			return
		}
		defer sess.Close()
		_ = sess.RequestPty("xterm", 24, 80, ssh.TerminalModes{})
		pr, pw := io.Pipe()
		sess.Stdout = pw
		sess.Stderr = pw
		if err := sess.Shell(); err != nil {
			return
		}
		sc := bufio.NewScanner(pr)
		buf := ""
		for sc.Scan() {
			buf += sc.Text() + "\n"
			if u := pickRelayURL(buf); u != "" {
				select {
				case got <- u:
				default:
				}
				return
			}
		}
	}()

	select {
	case url := <-got:
		sshMu.Lock()
		if sshClient != nil {
			_ = sshClient.Close()
		}
		sshClient = client
		sshMu.Unlock()
		setRelay(url)
		fmt.Println("Internet table (relay) →", url)
		return true
	case <-time.After(16 * time.Second):
		if _, relay, _ := netSnapshot(); relay != "" {
			sshMu.Lock()
			sshClient = client
			sshMu.Unlock()
			return true
		}
		_ = ln.Close()
		_ = client.Close()
		return false
	}
}

func startNativeSSH(port int) bool {
	hops := []sshHop{
		{addr: "a.pinggy.io:443", user: "free", remote: "0.0.0.0:0"},
		{addr: "free.pinggy.io:443", user: "free", remote: "0.0.0.0:0"},
		{addr: "localhost.run:22", user: "nokey", remote: "0.0.0.0:80"},
		{addr: "serveo.net:22", user: "serveo", remote: "0.0.0.0:80"},
	}
	for _, hop := range hops {
		fmt.Println("Internet table: opening a path friends can reach…")
		if tryNativeSSH(port, hop) {
			return true
		}
		closeSSH()
	}
	return false
}
