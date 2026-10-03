use crate::Result;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::{
    io::{Read, Write},
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant},
};
pub const LIMIT: usize = 256 * 1024 * 1024;
pub fn worker(file: &Path) -> Result<()> {
    let data = std::fs::read(file).map_err(|e| e.to_string())?;
    let out = decode_direct(&data)?;
    std::io::stdout()
        .write_all(out.as_bytes())
        .map_err(|e| e.to_string())
}
pub fn decode_direct(data: &[u8]) -> Result<String> {
    if data.len() > LIMIT {
        return Err("存档超过 256 MiB 限制".into());
    }
    if data.starts_with(b"SiiNunit") {
        return String::from_utf8(data.to_vec()).map_err(|e| e.to_string());
    }
    if !data.starts_with(b"ScsC") && !data.starts_with(b"BSII") {
        return Err("不支持的存档格式（需要 SiiNunit、ScsC 或 BSII）".into());
    }
    if data.starts_with(b"ScsC") && data.len() < 72 {
        return Err("加密存档头不完整".into());
    }
    let output = std::panic::catch_unwind(|| decrypt_truck::decrypt_bin_file(&data.to_vec()))
        .map_err(|_| "解码器遇到无法识别的数据，原存档未修改")??;
    if output.len() > LIMIT {
        return Err("解码结果超过 256 MiB 限制".into());
    }
    let text = String::from_utf8(output).map_err(|e| e.to_string())?;
    if !text.starts_with("SiiNunit") {
        return Err("解码结果不是 SiiNunit".into());
    }
    Ok(text)
}
pub fn read(file: &Path) -> Result<String> {
    let data = std::fs::read(file).map_err(|e| format!("读取 {} 失败：{e}", file.display()))?;
    if data.starts_with(b"SiiNunit") {
        return decode_direct(&data);
    }
    let mut command = Command::new(std::env::current_exe().map_err(|e| e.to_string())?);
    command
        .arg("--decode-worker")
        .arg(file)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    command.creation_flags(0x08000000);
    let mut child = command.spawn().map_err(|e| e.to_string())?;
    let out = child.stdout.take().unwrap();
    let err = child.stderr.take().unwrap();
    let reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        out.take((LIMIT + 1) as u64)
            .read_to_end(&mut buf)
            .map(|_| buf)
    });
    let errors = std::thread::spawn(move || {
        let mut buf = String::new();
        let _ = err.take(65536).read_to_string(&mut buf);
        buf
    });
    let start = Instant::now();
    let status = loop {
        if let Some(s) = child.try_wait().map_err(|e| e.to_string())? {
            break s;
        }
        if start.elapsed() > Duration::from_secs(30) {
            let _ = child.kill();
            let _ = child.wait();
            return Err("解密超时，原文件未修改".into());
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let buf = reader
        .join()
        .map_err(|_| "解密输出读取失败")?
        .map_err(|e| e.to_string())?;
    let error = errors.join().unwrap_or_default();
    if !status.success() {
        return Err(format!("解密进程失败：{error}"));
    }
    if buf.len() > LIMIT {
        return Err("解密输出过大".into());
    }
    String::from_utf8(buf).map_err(|e| e.to_string())
}
