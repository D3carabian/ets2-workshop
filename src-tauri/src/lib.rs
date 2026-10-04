pub mod catalog;
pub mod decoder;
pub mod discovery;
pub mod edit;
pub mod garage;
pub mod runtime;
pub mod setup;
pub mod sii;
pub mod storage;
pub type Result<T> = std::result::Result<T, String>;
pub fn hash(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(data))
}
