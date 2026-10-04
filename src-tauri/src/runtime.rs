use crate::Result;
pub fn installed() -> bool {
    #[cfg(windows)]
    {
        use winreg::{
            enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY},
            RegKey,
        };
        let path =
            "Software\\Microsoft\\EdgeUpdate\\Clients\\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}";
        // HKCU uses the current user's native key; the machine installation is
        // registered in the 32-bit view, including on 64-bit Windows.
        // https://learn.microsoft.com/microsoft-edge/webview2/concepts/distribution
        for (hive, flags) in [
            (HKEY_CURRENT_USER, KEY_READ),
            (HKEY_LOCAL_MACHINE, KEY_READ | KEY_WOW64_32KEY),
        ] {
            if let Ok(k) = RegKey::predef(hive).open_subkey_with_flags(path, flags) {
                if let Ok(v) = k.get_value::<String, _>("pv") {
                    if valid_version(&v) {
                        return true;
                    }
                }
            }
        }
    }
    false
}
fn valid_version(v: &str) -> bool {
    let pieces: Vec<_> = v.split('.').collect();
    pieces.len() == 4
        && pieces.iter().all(|p| {
            !p.is_empty() && p.bytes().all(|c| c.is_ascii_digit()) && p.parse::<u32>().is_ok()
        })
        && pieces.iter().any(|p| p.parse::<u32>().unwrap_or(0) > 0)
}
pub fn message(text: &str, question: bool) -> bool {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::*;
        let t: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
        let title: Vec<u16> = "ETS2 Workshop".encode_utf16().chain(Some(0)).collect();
        let flags = if question {
            MB_YESNO | MB_ICONINFORMATION
        } else {
            MB_OK | MB_ICONERROR
        };
        return unsafe { MessageBoxW(std::ptr::null_mut(), t.as_ptr(), title.as_ptr(), flags) }
            == IDYES;
    }
    #[cfg(not(windows))]
    {
        eprintln!("{text}");
        false
    }
}
const MANUAL_INSTALL_MESSAGE: &str = "缺少 Microsoft Edge WebView2 Runtime，暂时无法显示应用界面。\n\n请访问微软官方下载页面，安装 Evergreen Standalone Installer（x64）：\nhttps://developer.microsoft.com/microsoft-edge/webview2/#download-section\n\n安装完成后重新打开 ETS2 Workshop.exe。";

fn ensure_detected(is_installed: bool) -> Result<()> {
    if is_installed {
        Ok(())
    } else {
        Err(MANUAL_INSTALL_MESSAGE.into())
    }
}

pub fn ensure() -> Result<()> {
    ensure_detected(installed())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_detection() {
        for value in [
            "0.0.0.0",
            "",
            "x.0.0.0",
            "+130.0.2849.68",
            "130.0.2849",
            "130.0.2849.68.1",
            "130..2849.68",
            "130.0.2849.68 ",
            "4294967296.0.0.0",
        ] {
            assert!(!valid_version(value), "{value}");
        }
        assert!(valid_version("130.0.2849.68"));
    }

    #[test]
    fn missing_runtime_provides_manual_install_and_restart_instructions() {
        let error = ensure_detected(false).unwrap_err();
        assert!(error
            .contains("https://developer.microsoft.com/microsoft-edge/webview2/#download-section"));
        assert!(error.contains("Evergreen Standalone Installer（x64）"));
        assert!(error.contains("安装完成后重新打开 ETS2 Workshop.exe"));
        // Retrying performs detection again and succeeds once manually installed.
        assert!(ensure_detected(true).is_ok());
    }
}
