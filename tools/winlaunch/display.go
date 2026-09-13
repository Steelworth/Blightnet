package main

import (
	"strconv"
	"strings"
	"syscall"
	"unsafe"
)

type rect struct {
	Left, Top, Right, Bottom int32
}

type monitorInfoEx struct {
	CbSize  uint32
	Monitor rect
	Work    rect
	Flags   uint32
	Device  [32]uint16
}

type displayRec struct {
	Name    string `json:"name"`
	X       int    `json:"x"`
	Y       int    `json:"y"`
	W       int    `json:"w"`
	H       int    `json:"h"`
	On      bool   `json:"on"`
	Hz      int    `json:"hz,omitempty"`
	Primary bool   `json:"primary,omitempty"`
}

const (
	monitorInfoPrimary = 0x00000001
	swRestore          = 9
	swMaximize         = 3
	swpShowWindow      = 0x0040
)

var (
	user32                  = syscall.NewLazyDLL("user32.dll")
	procEnumDisplayMonitors = user32.NewProc("EnumDisplayMonitors")
	procGetMonitorInfo      = user32.NewProc("GetMonitorInfoW")
	procEnumWindows         = user32.NewProc("EnumWindows")
	procGetWindowTextW      = user32.NewProc("GetWindowTextW")
	procGetWindowTextLength = user32.NewProc("GetWindowTextLengthW")
	procIsWindowVisible     = user32.NewProc("IsWindowVisible")
	procSetWindowPos        = user32.NewProc("SetWindowPos")
	procShowWindow          = user32.NewProc("ShowWindow")
)

func displayName(device string, i int) string {
	d := strings.TrimPrefix(device, `\\.\`)
	if d == "" {
		return "Display " + strconv.Itoa(i)
	}
	if strings.HasPrefix(strings.ToUpper(d), "DISPLAY") {
		n := strings.TrimPrefix(strings.ToUpper(d), "DISPLAY")
		n = strings.TrimLeft(n, " _")
		if n != "" {
			return "Display " + n
		}
	}
	return d
}

func listDisplays() []displayRec {
	var out []displayRec
	n := 0
	cb := syscall.NewCallback(func(hMonitor, hdcMonitor, lprcMonitor, dwData uintptr) uintptr {
		var info monitorInfoEx
		info.CbSize = uint32(unsafe.Sizeof(info))
		r, _, _ := procGetMonitorInfo.Call(hMonitor, uintptr(unsafe.Pointer(&info)))
		if r == 0 {
			return 1
		}
		n++
		primary := info.Flags&monitorInfoPrimary != 0
		name := displayName(syscall.UTF16ToString(info.Device[:]), n)
		out = append(out, displayRec{
			Name:    name,
			X:       int(info.Monitor.Left),
			Y:       int(info.Monitor.Top),
			W:       int(info.Monitor.Right - info.Monitor.Left),
			H:       int(info.Monitor.Bottom - info.Monitor.Top),
			On:      primary,
			Primary: primary,
		})
		return 1
	})
	procEnumDisplayMonitors.Call(0, 0, cb, 0)
	if len(out) > 0 && !anyOn(out) {
		out[0].On = true
	}
	return out
}

func anyOn(rows []displayRec) bool {
	for _, r := range rows {
		if r.On {
			return true
		}
	}
	return false
}

func findDisplay(name string) *displayRec {
	want := strings.TrimSpace(strings.ToLower(name))
	rows := listDisplays()
	for i := range rows {
		if strings.ToLower(rows[i].Name) == want {
			rec := rows[i]
			return &rec
		}
	}
	return nil
}

func blightnetHWND() uintptr {
	var found uintptr
	cb := syscall.NewCallback(func(hwnd, lparam uintptr) uintptr {
		vis, _, _ := procIsWindowVisible.Call(hwnd)
		if vis == 0 {
			return 1
		}
		n, _, _ := procGetWindowTextLength.Call(hwnd)
		if n == 0 {
			return 1
		}
		buf := make([]uint16, n+1)
		procGetWindowTextW.Call(hwnd, uintptr(unsafe.Pointer(&buf[0])), uintptr(len(buf)))
		title := strings.ToLower(syscall.UTF16ToString(buf))
		if strings.Contains(title, "blightnet") {
			found = hwnd
			return 0
		}
		return 1
	})
	procEnumWindows.Call(cb, 0)
	return found
}

func placeOnDisplay(name string) *displayRec {
	rec := findDisplay(name)
	if rec == nil {
		return nil
	}
	hwnd := blightnetHWND()
	if hwnd != 0 {
		procShowWindow.Call(hwnd, swRestore)
		procSetWindowPos.Call(hwnd, 0, uintptr(rec.X), uintptr(rec.Y), uintptr(rec.W), uintptr(rec.H), swpShowWindow)
		procShowWindow.Call(hwnd, swMaximize)
	}
	rows := listDisplays()
	for i := range rows {
		rows[i].On = strings.EqualFold(rows[i].Name, rec.Name)
	}
	rec.On = true
	return rec
}
