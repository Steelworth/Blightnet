package main

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"syscall"
	"time"
)

func browserCandidates() []string {
	local := os.Getenv("LOCALAPPDATA")
	pf := os.Getenv("ProgramFiles")
	pf86 := os.Getenv("ProgramFiles(x86)")
	paths := []string{
		filepath.Join(pf, "Microsoft", "Edge", "Application", "msedge.exe"),
		filepath.Join(pf86, "Microsoft", "Edge", "Application", "msedge.exe"),
		filepath.Join(pf, "Google", "Chrome", "Application", "chrome.exe"),
		filepath.Join(pf86, "Google", "Chrome", "Application", "chrome.exe"),
		filepath.Join(local, "Google", "Chrome", "Application", "chrome.exe"),
		filepath.Join(pf, "BraveSoftware", "Brave-Browser", "Application", "brave.exe"),
		filepath.Join(pf86, "BraveSoftware", "Brave-Browser", "Application", "brave.exe"),
		filepath.Join(local, "BraveSoftware", "Brave-Browser", "Application", "brave.exe"),
	}
	out := []string{}
	seen := map[string]bool{}
	for _, p := range paths {
		if p == "" || seen[p] {
			continue
		}
		if st, err := os.Stat(p); err == nil && !st.IsDir() {
			seen[p] = true
			out = append(out, p)
		}
	}
	return out
}

func openBrowser(u string) {
	time.Sleep(300 * time.Millisecond)
	attr := &syscall.SysProcAttr{HideWindow: true}
	for _, exe := range browserCandidates() {
		cmd := exec.Command(exe, "--app="+u, "--new-window")
		cmd.SysProcAttr = attr
		if err := cmd.Start(); err == nil {
			fmt.Println("Opened", filepath.Base(exe))
			return
		}
	}
	cmd := exec.Command("cmd", "/C", "start", "", u)
	cmd.SysProcAttr = attr
	if err := cmd.Start(); err != nil {
		fmt.Println("Open this URL yourself:", u)
	}
}
