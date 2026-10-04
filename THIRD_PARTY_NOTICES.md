# Third-party notices

## DecryptTruck

This application includes and compiles DecryptTruck 1.3.7, by Fernando Garrido (SiberianCoffe), licensed under MIT.

Source: https://github.com/CoffeSiberian/DecryptTruck

Pinned upstream commit: `4b6a167d7b35a5234bb3dde17ac1532740860791`.

The complete license is included in `licenses/DecryptTruck-MIT.txt` and with the vendored source in `src-tauri/vendor/decrypt-truck/LICENSE`. Local changes are documented in that directory's `UPSTREAM.md`.

Upstream credits the format research and implementations in TheLazyTomcat/SII_Decrypt and SII DecryptSharp. The previous prototype's SII_Decrypt.dll is not included or required by this release.

## SCS Software Game Archive Extractor

The official extractor is **not included in this ZIP**. During first-run setup, the application retrieves it directly from SCS Software's official download server, verifies pinned archive and executable SHA256 hashes, and caches it locally. Game archives, models, textures, decrypted player saves and generated part catalogs are not distributed.

Official documentation: https://modding.scssoft.com/wiki/Documentation/Tools/Game_Archive_Extractor

## Microsoft Edge WebView2 Runtime

The launcher detects the installed Evergreen runtime. If missing, it displays Microsoft's official download page and asks the user to install the Evergreen Standalone Installer (x64), then restart the application. It does not download or execute a runtime installer. The runtime is provided and serviced by Microsoft under Microsoft's terms; it is not included in this ZIP.

https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution

## Application dependencies

The application also uses Tauri, React, TypeScript, Vite, Lucide, and Rust crates listed in Cargo.lock. Their dependency versions are locked in the source repository. The packaging workflow generates `licenses/dependencies/DEPENDENCIES.md`, `inventory.json`, and collected license texts in every release archive.


## Locale archive path hashing

The read-only game text reader uses the 2011 CityHash variant from the MIT-licensed [scs_tools cityhash module](https://github.com/Wilps93/scs_tools/blob/e147e755778579e78f80a406a53c64515113d7a1/src/cityhash.rs), pinned to commit `e147e755778579e78f80a406a53c64515113d7a1`. The scs_tools and underlying Google notices are included in `licenses/locale-cityhash-MIT.txt` and every release package. Archive metadata reading is implemented locally from documented format information.

ETS2 localized text is read from the user's installed game and cached locally for display. No game localization database or other game assets are included in the application download.
