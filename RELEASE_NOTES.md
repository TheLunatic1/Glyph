## v2.5.0 — Universal Protocol Tunneling

This release introduces a robust, low-level tunneling engine allowing you to forward any protocol seamlessly over your SSH connection.

### New Features

- **Universal Protocol Tunneling:**
  - **TCP & UDP Support:** Fully forward any TCP or UDP traffic from your local machine to any remote destination via your connected server.
  - **Custom Protocol Support:** By supporting both TCP and raw UDP, Glyph now supports practically any protocol (HTTP, HTTPS, FTP, DNS, custom game servers, etc.) out of the box!
  - **Bulletproof Architecture:** UDP tunneling uses a highly optimized, raw OS-level unbuffered streaming engine to guarantee zero data sticking and maximum throughput.
  - **Connection Resilience:** The tunneling engine safely handles and ignores dropped connections (e.g. background browser prefetches) without interrupting the main server.

