use flate2::read::ZlibDecoder;
use std::io::Read;
pub fn uncompress(source: &[u8]) -> Result<Vec<u8>, String> {
    const LIMIT: u64 = 256 * 1024 * 1024;
    let decoder = ZlibDecoder::new(source);
    let mut buffer = Vec::new();
    decoder
        .take(LIMIT + 1)
        .read_to_end(&mut buffer)
        .map_err(|e| e.to_string())?;
    if buffer.len() as u64 > LIMIT {
        return Err("Decompressed save exceeds 256 MiB".into());
    }
    Ok(buffer)
}
