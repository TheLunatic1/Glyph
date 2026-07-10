## v2.2.2 — Terminal Paste & Input Duplication Fixes

This patch resolves an issue where pasting text into the SSH terminal (`Ctrl+V` or right-click paste) could duplicate input or insert `^V` control characters.

---

### Bug Fixes & Improvements

- **Fixed Terminal Paste Duplication:** Removed redundant custom `Ctrl+V` clipboard listeners in `Terminal.jsx` so `xterm.js` and native browser paste events handle clipboard input cleanly without double-firing.
- **Fixed Control Character Insertion (`^V`):** Corrected custom key handler return flags to ensure `Ctrl+V` triggers native clipboard pasting instead of sending raw terminal ASCII control characters (`0x16` / `SYN`).
- **Electron Stability:** Updated Electron dependency configuration to v30.5.1.

---

