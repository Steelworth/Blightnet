package main

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"syscall"
)

func exeNewPath(root string) string {
	return filepath.Join(root, "Blightnet.exe.new")
}

func scheduleExeSwap() {
	root := updateRoot
	if root == "" {
		root = appRoot()
	}
	newer := exeNewPath(root)
	if _, err := os.Stat(newer); err != nil {
		return
	}
	dest := filepath.Join(root, "Blightnet.exe")
	script := fmt.Sprintf(`ping -n 3 127.0.0.1 >nul & move /Y "%s" "%s"`, newer, dest)
	cmd := exec.Command("cmd", "/C", script)
	cmd.SysProcAttr = &syscall.SysProcAttr{HideWindow: true}
	_ = cmd.Start()
}
