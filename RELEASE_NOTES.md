## v2.3.0 — The SFTP & Security Update

This major update transforms the SFTP tab into a fully featured File Explorer and introduces custom encryption for your exported server configurations!

### New Features

- **SFTP File Explorer Enhancements:**
  - **Drag & Drop Upload:** You can now instantly drag and drop files from your desktop straight into the SFTP window to upload them.
  - **Right-Click Context Menu:** Right-click any remote file or folder to access Copy, Cut, Rename, Delete, and Download options.
  - **Instant Remote Copy & Paste:** Copy or move files instantly using native server commands without needing to download and re-upload.
  - **Create Files & Folders:** New buttons in the header let you easily scaffold out folder structures directly on the server.
  - **Breadcrumb Navigation:** The top navigation bar now splits paths into clickable chunks for easy back-navigation.
- **Custom Encryption for Exports:** Server config backups are now securely encrypted with a custom algorithm and exported with the `.glyph` file extension.
- **Password Visibility:** Added a toggle (eye icon) to let you view your password while logging in.

### Bug Fixes & Improvements

- **Delete Server Modal:** Replaced the generic browser `window.confirm` popup with a beautiful custom modal for confirming server deletion.
- **Addes Edit server credentials feature:** Added the ability to edit server credentials and updated the server list table.
