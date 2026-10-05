## v2.7.6 — Mobile App Promotion & Dynamic APK QR Code System

### What's New & Improvements

- **Glyph Mobile Promotion & QR Code Distribution:** Integrated an in-app distribution modal (`MobileAppModal.jsx`) allowing users to scan a dynamic QR code directly with their phone camera to instantly download the latest Android release APK (`Glyph-Mobile-Android-v1.0.1.apk` / `app-release.apk`).
- **Live GitHub Releases API & Dynamic QR Engine:** Built a client-side dynamic QR engine powered by the GitHub Releases API (`TheLunatic1/glyph-app`) that auto-fetches the latest release version, APK download link, file size, and release date in real time without requiring desktop app updates.
- **QR Mode Switcher & Direct Download:** Toggle QR code scanning between direct APK sideload binary download and the full GitHub release page, complete with 1-click PC download and link copy actions.
- **Cross-Platform Promotional Navigation:** Added convenient access points across the desktop app:
  - Top header toolbar button (`[📱 Mobile App | APK]`) in the Saved Servers view.
  - Server navigation action button (`Glyph Mobile (APK)`) in the connected server sidebar.
  - Dedicated `Glyph Mobile (Android)` tab in the Settings modal with feature previews and direct links.
- **Ecosystem Parity & Documentation:** Documented complete `.glyph` AES-256-GCM vault backup compatibility between Desktop and Mobile apps.
