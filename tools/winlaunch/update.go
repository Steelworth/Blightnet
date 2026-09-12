package main

import (
	"crypto/sha1"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"sync"
	"time"
)

const (
	githubRepo   = "Steelworth/Blightnet"
	githubBranch = "main"
	githubAPI    = "https://api.github.com"
	updateUA     = "Blightnet-Updater"
)

var updateSkipPrefix = []string{
	"uploads/",
	".git/",
	"tools/_go/",
	"tools/_raw/",
	"tools/_wav/",
	"__pycache__/",
}

var updateServerFiles = map[string]bool{
	"serve.py":            true,
	"blightnet_window.py": true,
	"browser.py":          true,
	"start.sh":            true,
	"start.bat":           true,
	"Blightnet.exe":       true,
	"Hearthsong.exe":      true,
}

type updateJob struct {
	Running bool     `json:"running"`
	Phase   string   `json:"phase"`
	Message string   `json:"message"`
	Checked int      `json:"checked"`
	Changed int      `json:"changed"`
	Files   []string `json:"files"`
	SHA     string   `json:"sha"`
	Reload  bool     `json:"reload"`
	Restart bool     `json:"restart"`
	Error   string   `json:"error"`
	OK      bool     `json:"ok"`
}

var (
	updateMu   sync.Mutex
	updateRoot string
	updateSt   = updateJob{Phase: "idle", Files: []string{}}
)

func initUpdate(root string) {
	updateRoot = root
}

func updateSnapshot() updateJob {
	updateMu.Lock()
	defer updateMu.Unlock()
	out := updateSt
	if out.Files == nil {
		out.Files = []string{}
	}
	out.OK = out.Error == ""
	return out
}

func updateSet(fn func(*updateJob)) {
	updateMu.Lock()
	defer updateMu.Unlock()
	fn(&updateSt)
}

func startUpdate(root string) updateJob {
	if root != "" {
		updateRoot = root
	}
	updateMu.Lock()
	if updateSt.Running {
		st := updateSt
		updateMu.Unlock()
		st.OK = true
		return st
	}
	updateSt = updateJob{
		Running: true,
		Phase:   "checking",
		Message: "Checking GitHub…",
		Files:   []string{},
		OK:      true,
	}
	updateMu.Unlock()
	go runUpdateJob()
	return updateSnapshot()
}

func gitBlobSHA(data []byte) string {
	h := sha1.New()
	fmt.Fprintf(h, "blob %d\x00", len(data))
	h.Write(data)
	return hex.EncodeToString(h.Sum(nil))
}

func updateSafeRel(rel string) string {
	rel = strings.ReplaceAll(rel, "\\", "/")
	rel = strings.TrimPrefix(rel, "/")
	if rel == "" || rel == "." || rel == ".." || strings.HasPrefix(rel, "../") || strings.Contains(rel, "/../") || strings.Contains(rel, "\x00") {
		return ""
	}
	lower := strings.ToLower(rel)
	for _, p := range updateSkipPrefix {
		if lower == strings.TrimSuffix(p, "/") || strings.HasPrefix(lower, p) {
			return ""
		}
	}
	for _, part := range strings.Split(rel, "/") {
		if part == "__pycache__" || strings.HasSuffix(part, ".pyc") {
			return ""
		}
	}
	base := filepath.Base(rel)
	if base == ".blightnet-sha" || strings.HasSuffix(base, ".blightnet-new") {
		return ""
	}
	return rel
}

func shaPath() string {
	return filepath.Join(updateRoot, ".blightnet-sha")
}

func readSavedSHA() string {
	b, err := os.ReadFile(shaPath())
	if err != nil {
		return ""
	}
	return strings.TrimSpace(string(b))
}

func writeSavedSHA(sha string) {
	_ = os.WriteFile(shaPath(), []byte(sha+"\n"), 0644)
}

func httpGet(url string, timeout time.Duration) ([]byte, error) {
	client := &http.Client{Timeout: timeout}
	req, err := http.NewRequest(http.MethodGet, url, nil)
	if err != nil {
		return nil, err
	}
	req.Header.Set("User-Agent", updateUA)
	req.Header.Set("Accept", "application/vnd.github+json, application/octet-stream, */*")
	resp, err := client.Do(req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()
	if resp.StatusCode >= 300 {
		return nil, fmt.Errorf("GitHub returned %d", resp.StatusCode)
	}
	return io.ReadAll(resp.Body)
}

func httpJSON(url string, dest any) error {
	b, err := httpGet(url, 45*time.Second)
	if err != nil {
		return err
	}
	return json.Unmarshal(b, dest)
}

func runGit(args ...string) (string, error) {
	if updateRoot == "" {
		return "", fmt.Errorf("no root")
	}
	if _, err := os.Stat(filepath.Join(updateRoot, ".git")); err != nil {
		return "", err
	}
	cmd := exec.Command("git", args...)
	cmd.Dir = updateRoot
	cmd.Env = append(os.Environ(), "GIT_TERMINAL_PROMPT=0")
	out, err := cmd.CombinedOutput()
	return strings.TrimSpace(string(out)), err
}

func writeRel(rel string, data []byte) error {
	dest := filepath.Join(append([]string{updateRoot}, strings.Split(rel, "/")...)...)
	if err := os.MkdirAll(filepath.Dir(dest), 0755); err != nil {
		return err
	}
	tmp := dest + ".blightnet-new"
	if err := os.WriteFile(tmp, data, 0644); err != nil {
		return err
	}
	if err := os.Rename(tmp, dest); err != nil {
		_ = os.Remove(tmp)
		return err
	}
	return nil
}

func tryGitUpdate() bool {
	if _, err := runGit("rev-parse", "--is-inside-work-tree"); err != nil {
		return false
	}
	updateSet(func(j *updateJob) { j.Phase = "checking"; j.Message = "Checking GitHub…" })
	oldSHA, _ := runGit("rev-parse", "HEAD")
	_, err := runGit("fetch", "--quiet", "https://github.com/"+githubRepo+".git", githubBranch)
	if err != nil {
		_, err = runGit("fetch", "--quiet", "origin", githubBranch)
	}
	if err != nil {
		return false
	}
	newSHA, err := runGit("rev-parse", "FETCH_HEAD")
	if err != nil || newSHA == "" {
		return false
	}
	if oldSHA != "" && oldSHA == newSHA {
		updateSet(func(j *updateJob) {
			j.Phase = "done"
			j.Message = "Up to date"
			j.SHA = newSHA
			j.Changed = 0
			j.Files = []string{}
		})
		writeSavedSHA(newSHA)
		return true
	}
	names := []string{}
	if oldSHA != "" {
		if diff, err := runGit("diff", "--name-only", oldSHA, newSHA); err == nil && diff != "" {
			for _, ln := range strings.Split(diff, "\n") {
				if s := strings.TrimSpace(ln); s != "" {
					names = append(names, s)
				}
			}
		}
	}
	updateSet(func(j *updateJob) { j.Phase = "downloading"; j.Message = "Downloading updates…" })
	if _, err := runGit("merge", "--ff-only", newSHA); err != nil {
		return false
	}
	safe := []string{}
	restart := false
	for _, n := range names {
		if updateSafeRel(n) == "" {
			continue
		}
		safe = append(safe, n)
		if updateServerFiles[n] || strings.HasPrefix(n, "gst/") {
			restart = true
		}
	}
	msg := "Up to date"
	if len(safe) == 1 {
		msg = "Updated 1 file"
	} else if len(safe) > 1 {
		msg = fmt.Sprintf("Updated %d files", len(safe))
	}
	clipped := safe
	if len(clipped) > 80 {
		clipped = clipped[:80]
	}
	updateSet(func(j *updateJob) {
		j.Phase = "done"
		j.Message = msg
		j.SHA = newSHA
		j.Changed = len(safe)
		j.Files = clipped
		j.Reload = len(safe) > 0 && !restart
		j.Restart = restart
	})
	writeSavedSHA(newSHA)
	return true
}

type ghCommit struct {
	SHA string `json:"sha"`
}

type ghTree struct {
	Tree []struct {
		Path string `json:"path"`
		Type string `json:"type"`
		SHA  string `json:"sha"`
	} `json:"tree"`
}

func githubHeadSHA() (string, error) {
	var c ghCommit
	err := httpJSON(githubAPI+"/repos/"+githubRepo+"/commits/"+githubBranch, &c)
	if err != nil {
		return "", err
	}
	if len(c.SHA) < 7 {
		return "", fmt.Errorf("GitHub did not return a commit")
	}
	return c.SHA, nil
}

func tryAPIUpdate() error {
	updateSet(func(j *updateJob) { j.Phase = "checking"; j.Message = "Checking GitHub…" })
	head, err := githubHeadSHA()
	if err != nil {
		return err
	}
	current := readSavedSHA()
	if local, err := runGit("rev-parse", "HEAD"); err == nil && local != "" {
		current = local
	}
	if current != "" && current == head {
		updateSet(func(j *updateJob) {
			j.Phase = "done"
			j.Message = "Up to date"
			j.SHA = head
			j.Changed = 0
			j.Files = []string{}
		})
		writeSavedSHA(head)
		return nil
	}
	var tree ghTree
	if err := httpJSON(githubAPI+"/repos/"+githubRepo+"/git/trees/"+head+"?recursive=1", &tree); err != nil {
		return err
	}
	changed := []string{}
	restart := false
	total := len(tree.Tree)
	checked := 0
	for _, e := range tree.Tree {
		if e.Type != "blob" {
			continue
		}
		rel := updateSafeRel(e.Path)
		if rel == "" {
			continue
		}
		checked++
		if checked%25 == 0 || checked == total {
			n := checked
			t := total
			updateSet(func(j *updateJob) {
				j.Checked = n
				j.Message = fmt.Sprintf("Checking %d/%d…", n, t)
			})
		}
		dest := filepath.Join(append([]string{updateRoot}, strings.Split(rel, "/")...)...)
		have := ""
		if b, err := os.ReadFile(dest); err == nil {
			have = gitBlobSHA(b)
		}
		if have != "" && e.SHA != "" && have == e.SHA {
			continue
		}
		path := rel
		updateSet(func(j *updateJob) {
			j.Phase = "downloading"
			j.Message = "Downloading " + path + "…"
			j.Checked = checked
		})
		url := "https://raw.githubusercontent.com/" + githubRepo + "/" + head + "/" + strings.ReplaceAll(rel, " ", "%20")
		data, err := httpGet(url, 90*time.Second)
		if err != nil {
			return err
		}
		if err := writeRel(rel, data); err != nil {
			return err
		}
		changed = append(changed, rel)
		if updateServerFiles[rel] || strings.HasPrefix(rel, "gst/") {
			restart = true
		}
		clip := changed
		if len(clip) > 80 {
			clip = clip[:80]
		}
		nch := len(changed)
		updateSet(func(j *updateJob) {
			j.Changed = nch
			j.Files = clip
		})
	}
	writeSavedSHA(head)
	msg := "Up to date"
	if len(changed) == 1 {
		msg = "Updated 1 file"
	} else if len(changed) > 1 {
		msg = fmt.Sprintf("Updated %d files", len(changed))
	}
	clip := changed
	if len(clip) > 80 {
		clip = clip[:80]
	}
	updateSet(func(j *updateJob) {
		j.Phase = "done"
		j.Message = msg
		j.SHA = head
		j.Changed = len(changed)
		j.Files = clip
		j.Reload = len(changed) > 0 && !restart
		j.Restart = restart
	})
	return nil
}

func runUpdateJob() {
	defer func() {
		updateSet(func(j *updateJob) {
			j.Running = false
			if j.Phase == "checking" || j.Phase == "downloading" || j.Phase == "applying" || j.Phase == "" {
				j.Phase = "error"
				if j.Error == "" {
					j.Error = "Update stopped"
				}
				j.Message = j.Error
			}
		})
	}()
	if tryGitUpdate() {
		return
	}
	if err := tryAPIUpdate(); err != nil {
		msg := err.Error()
		if strings.Contains(msg, "403") {
			msg = "GitHub rate limit. Try again in a few minutes."
		}
		updateSet(func(j *updateJob) {
			j.Phase = "error"
			j.Error = msg
			j.Message = msg
		})
	}
}
