use crate::Result;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::{
    io::Read,
    path::Path,
    process::Command,
    time::{Duration, Instant},
};
pub fn installed() -> bool {
    #[cfg(windows)]
    {
        use winreg::{
            enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY},
            RegKey,
        };
        let path =
            "Software\\Microsoft\\EdgeUpdate\\Clients\\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}";
        for hive in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
            if let Ok(k) =
                RegKey::predef(hive).open_subkey_with_flags(path, KEY_READ | KEY_WOW64_32KEY)
            {
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
        && pieces.iter().all(|p| p.parse::<u32>().is_ok())
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
fn microsoft_signed(path: &Path) -> Result<()> {
    let script="$s=Get-AuthenticodeSignature -LiteralPath $env:ETS2_WEBVIEW_INSTALLER; if($s.Status -eq 'Valid' -and $s.SignerCertificate.Subject -match '(^|, )O=Microsoft Corporation(,|$)'){exit 0}else{exit 1}";
    let powershell = std::path::PathBuf::from(
        std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into()),
    )
    .join("System32/WindowsPowerShell/v1.0/powershell.exe");
    let mut cmd = Command::new(powershell);
    cmd.env_remove("PSModulePath");
    cmd.args(["-NoProfile", "-NonInteractive", "-Command", script])
        .env("ETS2_WEBVIEW_INSTALLER", path);
    #[cfg(windows)]
    cmd.creation_flags(0x08000000);
    if !cmd.status().map_err(|e| e.to_string())?.success() {
        return Err("微软运行时安装器签名验证失败，未执行安装。".into());
    }
    Ok(())
}
pub fn download_installer() -> Result<std::path::PathBuf> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(90))
        .build()
        .map_err(|e| e.to_string())?;
    let response = client
        .get("https://go.microsoft.com/fwlink/p/?LinkId=2124703")
        .send()
        .and_then(|r| r.error_for_status())
        .map_err(|e| format!("下载微软运行时失败：{e}"))?;
    let mut bytes = Vec::new();
    response
        .take(16 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 16 * 1024 * 1024 || !bytes.starts_with(b"MZ") {
        return Err("运行时安装器文件异常".into());
    }
    let dir = crate::storage::app_dir().join("tools");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!(
        "WebView2Setup-{}.exe",
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    if let Err(e) = microsoft_signed(&path) {
        let _ = std::fs::remove_file(&path);
        return Err(e);
    }
    Ok(path)
}
pub fn ensure() -> Result<()> {
    if installed() {
        return Ok(());
    }
    if !message("需要 Microsoft Edge WebView2 Runtime 才能显示界面。\n\n是否从微软官方下载并安装？需要联网，可能需要几分钟。安装完成后会自动打开首次启动向导。",true){return Err("运行时准备已取消。".into());}
    let path = download_installer()?;
    let result = (|| {
        let mut cmd = Command::new(&path);
        cmd.args(["/silent", "/install"]);
        #[cfg(windows)]
        cmd.creation_flags(0x08000000);
        let mut child = cmd.spawn().map_err(|e| e.to_string())?;
        let start = Instant::now();
        loop {
            if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
                if !status.success() && !installed() {
                    return Err(format!("运行时安装失败：{status}"));
                }
                break;
            }
            if start.elapsed() > Duration::from_secs(600) {
                return Err("运行时安装仍在进行，请稍后重新打开程序。".into());
            }
            std::thread::sleep(Duration::from_millis(250));
        }
        if !installed() {
            return Err("尚未检测到运行时，请完成微软安装器后重新打开程序。".into());
        }
        Ok(())
    })();
    let _ = std::fs::remove_file(path);
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn version_detection() {
        assert!(!valid_version("0.0.0.0"));
        assert!(!valid_version(""));
        assert!(!valid_version("x.0.0.0"));
        assert!(valid_version("130.0.2849.68"));
    }
}
