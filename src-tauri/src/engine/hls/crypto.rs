use aes::Aes128;
use cbc::Decryptor;
use cbc::cipher::{BlockDecryptMut, KeyIvInit};

type Aes128CbcDec = Decryptor<Aes128>;

pub struct HlsDecryptor {
    key: [u8; 16],
    default_iv: Option<[u8; 16]>,
}

impl HlsDecryptor {
    pub async fn new(
        client: &super::super::http::VideoClient,
        key_uri: &str,
        default_iv: Option<Vec<u8>>,
    ) -> Result<Self, String> {
        let key_bytes = client.get_bytes(key_uri).await?;
        if key_bytes.len() != 16 {
            return Err(format!("AES key is {} bytes, expected 16", key_bytes.len()));
        }
        let mut key = [0u8; 16];
        key.copy_from_slice(&key_bytes);

        let iv = default_iv.map(|v| {
            let mut arr = [0u8; 16];
            let len = v.len().min(16);
            // Right-align the IV bytes (big-endian) per RFC 8216 §5.2
            arr[16 - len..].copy_from_slice(&v[..len]);
            arr
        });

        Ok(Self { key, default_iv: iv })
    }

    pub fn decrypt(&self, data: &[u8], segment_iv: Option<&[u8]>, sequence: u64) -> Result<Vec<u8>, String> {
        let iv = if let Some(seg_iv) = segment_iv {
            let mut arr = [0u8; 16];
            let len = seg_iv.len().min(16);
            // Right-align per RFC 8216 §5.2
            arr[16 - len..].copy_from_slice(&seg_iv[..len]);
            arr
        } else if let Some(default) = self.default_iv {
            default
        } else {
            // Per RFC 8216 §4.3.2.4: use segment sequence number as IV
            // The sequence number is placed as a big-endian 128-bit integer
            sequence_to_iv(sequence)
        };

        let mut buf = data.to_vec();
        let decryptor = Aes128CbcDec::new(&self.key.into(), &iv.into());

        let decrypted = decryptor
            .decrypt_padded_mut::<block_padding::Pkcs7>(&mut buf)
            .map_err(|e| format!("AES decryption failed: {}", e))?;

        Ok(decrypted.to_vec())
    }
}

/// Convert segment sequence number to a 16-byte big-endian IV.
/// Per RFC 8216 §4.3.2.4, the sequence number fills the lower 8 bytes.
fn sequence_to_iv(seq: u64) -> [u8; 16] {
    let mut iv = [0u8; 16];
    iv[8..16].copy_from_slice(&seq.to_be_bytes());
    iv
}
