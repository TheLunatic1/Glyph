## v2.2.1 — Windows Auto-Update Fixes & SmartScreen Notice

You no longer need to visit GitHub to update Glyph. Updates now happen entirely inside the app!

> [!NOTE]
> **Windows SmartScreen Notice:**
> Because Glyph is an open-source application built without a paid corporate code-signing certificate, Microsoft Defender SmartScreen may display a *"Windows protected your PC"* popup when launching the installer or updating.
> 
> To proceed safely, simply click **More info** → **Run anyway**. Glyph is 100% clean, transparent, and open-source!

---

### New Feature: In-App Auto-Update

Glyph now checks for new releases automatically in the background when it starts up. When a new version is found, a banner appears at the top of the app with everything you need.

**For users who installed via the Setup `.exe` / AppImage / DMG:**

A purple banner will appear:

> Glyph v2.x.x is available! `[View Release Notes & Update]` `[Download Manually]`

- Click **"View Release Notes & Update"** to open the update modal
- See the full changelog for the new version
- Click **"Download & Install"** — a progress bar tracks the download in real time
- Once complete, click **"Restart & Install"** — Glyph closes, updates, and reopens automatically

**For portable `.exe` users:**

Click **"Download Manually"** to go directly to the GitHub Releases page and grab the latest portable build. Your data and saved servers are never affected.

---

### CI/CD

- Redesigned GitHub Actions release pipeline into a single unified workflow
- Bumping the version in `package.json` and pushing to `main` now automatically creates the release tag and kicks off the full Windows / Linux / macOS build pipeline
- The manual **"Run workflow"** button is still available in GitHub Actions for re-releasing or hotfixing

---

_Made by [TheLunatic1 (Salman Toha)](https://github.com/TheLunatic1)_
