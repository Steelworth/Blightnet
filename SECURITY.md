# Security

Blightnet is a **local** table. The server binds on your LAN so other computers at the same Wi-Fi can join. It is not meant to be opened to the public internet.

- Allow the firewall prompt on a **private** network only.
- Internet tables are **daemon-to-daemon**: UDP hole punch plus optional UPnP. There is no Cloudflare tunnel. Do not install cloudflared. If punch fails, forward TCP/UDP **8766** on the host router.
- Host-shared audio and maps stay on the host machine; chat pictures and voice go peer-to-peer and are not stored on the hub.

If you find a bug that lets a guest write outside `uploads/`, crash the hub, or read another table’s files, open a GitHub issue. Do not attach session logs that contain player handles or uploaded files unless you have stripped them.
