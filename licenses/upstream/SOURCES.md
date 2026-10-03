# Supplemental dependency license sources

These texts fill omissions in upstream crate archives. Original bytes and copyright notices are preserved. `scripts/licenses.mjs` verifies their SHA-256 before packaging and applies them only to the listed dependency versions. Source commits below come from each published crate's `.cargo_vcs_info.json`, rather than an assumed current branch.

| Crate | Version | Published source commit | License source |
| --- | --- | --- | --- |
| alloc-stdlib | 0.3.0 | 0a81fd6928ea3b33c8cd484aa4575d50ffb98012 | [Dropbox repository LICENSE](https://github.com/dropbox/rust-alloc-no-stdlib/blob/0a81fd6928ea3b33c8cd484aa4575d50ffb98012/LICENSE) |
| defmt-parser | 1.0.0 | 4a8cdb44891ed57b8ff5a023b6bec7137c48708f | [LICENSE-MIT](https://github.com/knurling-rs/defmt/blob/4a8cdb44891ed57b8ff5a023b6bec7137c48708f/LICENSE-MIT) and [LICENSE-APACHE](https://github.com/knurling-rs/defmt/blob/4a8cdb44891ed57b8ff5a023b6bec7137c48708f/LICENSE-APACHE) |
| webview2-com, webview2-com-sys | 0.39.1 | edc2caf886175ccaebe86078c9cfe1ae2a187328 | [Repository LICENSE](https://github.com/wravery/webview2-rs/blob/edc2caf886175ccaebe86078c9cfe1ae2a187328/LICENSE) |
| webview2-com-macros | 0.8.1 | dffa41a8a46d3f5565eefbff2de57d38d399f158 | [Repository LICENSE](https://github.com/wravery/webview2-rs/blob/dffa41a8a46d3f5565eefbff2de57d38d399f158/LICENSE) |
| selectors | 0.38.0 | 572ecba2d1600e7c3d490586692a209faf703baa | [Source header](https://github.com/servo/stylo/blob/572ecba2d1600e7c3d490586692a209faf703baa/selectors/lib.rs) explicitly refers to Mozilla's MPL 2.0. That repository commit does not contain the MPL full text. The included text is from [Mozilla's official plain-text MPL 2.0](https://www.mozilla.org/media/MPL/2.0/index.815ca599c9df.txt). |

The npm platform packages `@esbuild/win32-x64@0.25.12`, `@rollup/rollup-win32-x64-gnu@4.64.0`, `@rollup/rollup-win32-x64-msvc@4.64.0`, and `@tauri-apps/cli-win32-x64-msvc@2.12.1` use the license texts supplied with the matching-version `esbuild`, `rollup`, and `@tauri-apps/cli` parent packages. The collector checks exact parent names and versions, copies those original files, and records the parent in each report entry. Those texts are generated from `node_modules` during packaging rather than duplicated here.

For MPL-covered source, the unmodified selectors source corresponding to the dependency is available in the [official source tree](https://github.com/servo/stylo/tree/572ecba2d1600e7c3d490586692a209faf703baa/selectors) and the [published crate source archive](https://crates.io/api/v1/crates/selectors/0.38.0/download).
