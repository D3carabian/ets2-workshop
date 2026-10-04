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
fn read_limited(reader: impl Read, limit: usize) -> std::io::Result<Vec<u8>> {
    let bound = u64::try_from(limit)
        .ok()
        .and_then(|n| n.checked_add(1))
        .ok_or_else(|| std::io::Error::other("读取大小限制无效"))?;
    let mut data = Vec::new();
    reader.take(bound).read_to_end(&mut data)?;
    if data.len() > limit {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "存档超过读取大小限制",
        ));
    }
    Ok(data)
}
pub fn read_bounded(file: &Path) -> Result<Vec<u8>> {
    let reader =
        std::fs::File::open(file).map_err(|e| format!("读取 {} 失败：{e}", file.display()))?;
    read_limited(reader, LIMIT).map_err(|e| format!("读取 {} 失败：{e}", file.display()))
}
pub fn worker(file: &Path) -> Result<()> {
    let data = read_bounded(file)?;
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
    let data = read_bounded(file)?;
    if data.starts_with(b"SiiNunit") {
        return decode_direct(&data);
    }
    drop(data);
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
    let reader = std::thread::spawn(move || read_limited(out, LIMIT));
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
    String::from_utf8(buf).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_reader_consumes_at_most_limit_plus_one() {
        struct CountingReader {
            remaining: usize,
            consumed: usize,
        }
        impl Read for CountingReader {
            fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
                let n = buf.len().min(self.remaining);
                buf[..n].fill(b'x');
                self.remaining -= n;
                self.consumed += n;
                Ok(n)
            }
        }
        for (limit, size) in [(0, 1000), (8, 1000), (8, 8), (8, 7)] {
            let mut reader = CountingReader {
                remaining: size,
                consumed: 0,
            };
            let result = read_limited(&mut reader, limit);
            assert_eq!(reader.consumed, size.min(limit + 1));
            if size > limit {
                assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::InvalidData);
            } else {
                assert_eq!(result.unwrap().len(), size);
            }
        }
    }
}
