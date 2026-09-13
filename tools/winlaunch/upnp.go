package main

import (
	"encoding/xml"
	"fmt"
	"io"
	"net"
	"net/http"
	"strings"
	"time"
)

func outboundLAN() string {
	c, err := net.DialTimeout("udp", "1.1.1.1:80", 2*time.Second)
	if err != nil {
		return ""
	}
	defer c.Close()
	host, _, _ := net.SplitHostPort(c.LocalAddr().String())
	if host == "" || strings.HasPrefix(host, "127.") {
		return ""
	}
	return host
}

func upnpDiscover() string {
	conn, err := net.ListenUDP("udp4", &net.UDPAddr{Port: 0})
	if err != nil {
		return ""
	}
	defer conn.Close()
	msg := "M-SEARCH * HTTP/1.1\r\nHOST: 239.255.255.250:1900\r\nMAN: \"ssdp:discover\"\r\nMX: 2\r\nST: urn:schemas-upnp-org:device:InternetGatewayDevice:1\r\n\r\n"
	dst := &net.UDPAddr{IP: net.ParseIP("239.255.255.250"), Port: 1900}
	_, _ = conn.WriteToUDP([]byte(msg), dst)
	_ = conn.SetReadDeadline(time.Now().Add(1600 * time.Millisecond))
	buf := make([]byte, 4096)
	n, _, err := conn.ReadFromUDP(buf)
	if err != nil || n == 0 {
		return ""
	}
	for _, line := range strings.Split(string(buf[:n]), "\n") {
		low := strings.ToLower(strings.TrimSpace(line))
		if strings.HasPrefix(low, "location:") {
			return strings.TrimSpace(line[strings.Index(line, ":")+1:])
		}
	}
	return ""
}

type upnpService struct {
	ServiceType string `xml:"serviceType"`
	ControlURL  string `xml:"controlURL"`
}

func upnpControl(loc string) (svcType, action string) {
	client := &http.Client{Timeout: 2 * time.Second}
	resp, err := client.Get(loc)
	if err != nil {
		return "", ""
	}
	defer resp.Body.Close()
	body, _ := io.ReadAll(io.LimitReader(resp.Body, 1<<20))
	var doc struct {
		Services []upnpService `xml:"device>serviceList>service"`
		Nested   []upnpService `xml:"device>deviceList>device>serviceList>service"`
	}
	_ = xml.Unmarshal(body, &doc)
	svcs := append(doc.Services, doc.Nested...)
	var st, ctrl string
	for _, s := range svcs {
		if strings.Contains(s.ServiceType, "WANIPConnection") || strings.Contains(s.ServiceType, "WANPPPConnection") {
			st, ctrl = s.ServiceType, s.ControlURL
			break
		}
	}
	if st == "" || ctrl == "" {
		raw := string(body)
		if strings.Contains(raw, "WANIPConnection") {
			st = "urn:schemas-upnp-org:service:WANIPConnection:1"
		} else if strings.Contains(raw, "WANPPPConnection") {
			st = "urn:schemas-upnp-org:service:WANPPPConnection:1"
		}
		if i := strings.Index(raw, "<controlURL>"); i >= 0 {
			rest := raw[i+12:]
			if j := strings.Index(rest, "</controlURL>"); j >= 0 {
				ctrl = rest[:j]
			}
		}
	}
	if st == "" || ctrl == "" {
		return "", ""
	}
	if strings.HasPrefix(ctrl, "http") {
		return st, ctrl
	}
	u := loc
	if strings.HasPrefix(ctrl, "/") {
		schemeEnd := strings.Index(u, "://")
		slash := strings.Index(u[schemeEnd+3:], "/")
		if slash >= 0 {
			u = u[:schemeEnd+3+slash]
		}
		return st, u + ctrl
	}
	base := u
	if i := strings.LastIndex(base, "/"); i > 8 {
		base = base[:i]
	}
	return st, base + "/" + ctrl
}

func upnpMap(port int) bool {
	loc := upnpDiscover()
	if loc == "" {
		return false
	}
	st, action := upnpControl(loc)
	if action == "" {
		return false
	}
	lan := outboundLAN()
	if lan == "" {
		ips := lanIPs()
		if len(ips) == 0 {
			return false
		}
		lan = ips[0]
	}
	body := fmt.Sprintf(`<?xml version="1.0"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/" s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/">
<s:Body><u:AddPortMapping xmlns:u="%s">
<NewRemoteHost></NewRemoteHost>
<NewExternalPort>%d</NewExternalPort>
<NewProtocol>TCP</NewProtocol>
<NewInternalPort>%d</NewInternalPort>
<NewInternalClient>%s</NewInternalClient>
<NewEnabled>1</NewEnabled>
<NewPortMappingDescription>Blightnet</NewPortMappingDescription>
<NewLeaseDuration>86400</NewLeaseDuration>
</u:AddPortMapping></s:Body></s:Envelope>`, st, port, port, lan)
	req, err := http.NewRequest(http.MethodPost, action, strings.NewReader(body))
	if err != nil {
		return false
	}
	req.Header.Set("Content-Type", `text/xml; charset="utf-8"`)
	req.Header.Set("SOAPAction", `"`+st+`#AddPortMapping"`)
	client := &http.Client{Timeout: 2500 * time.Millisecond}
	resp, err := client.Do(req)
	if err != nil {
		return false
	}
	defer resp.Body.Close()
	return resp.StatusCode >= 200 && resp.StatusCode < 300
}

func setUPnP(on bool) {
	netInfo.mu.Lock()
	netInfo.upnp = on
	netInfo.mu.Unlock()
}
