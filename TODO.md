# QuickConvert — TODO & feature requests

Track planned work and ideas. Open a GitHub issue to discuss or claim an item.

## Feature requests

- [ ] **Windows code signing** — Sign `quickconvert.exe` (and the installer if we add one) so Windows SmartScreen and enterprise policies trust the build. Likely needs an Authenticode certificate (e.g. via SSL.com, DigiCert, or Azure Trusted Signing) and a CI step using `signtool` or `osslsigncode` before uploading release assets.

## TODO

- [ ] Document code-signing setup in README once implemented.
- [ ] Optional: Windows installer (MSI/Inno Setup) in addition to the portable zip.

## Done

- [x] Linux (KDE) desktop entry and GTK4/libadwaita UI
- [x] Windows portable zip via GitHub Actions (MSYS2 / GTK4)
- [x] GitHub Releases on version tags
- [x] System tray + minimize on close / Esc (Linux)
- [x] KDE global shortcut (Meta+Shift+C)
