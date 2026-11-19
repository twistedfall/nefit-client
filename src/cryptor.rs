use std::iter;

use aes::Aes256;
use aes::cipher::{BlockModeDecrypt, BlockModeEncrypt, BlockSizeUser, Key, KeyInit};
use base64::prelude::*;
use block_padding::ZeroPadding;
use ecb::{Decryptor, Encryptor};
use md5::{Digest, Md5};

use crate::{CryptError, Result};

const KEY_SALT: &[u8; 32] = b"\x58\xf1\x8d\x70\xf6\x67\xc9\xc7\x9e\xf7\xde\x43\x5b\xf0\xf9\xb1\x55\x3b\xbb\x6e\x61\x81\x62\x12\xab\x80\xe5\xb0\xd3\x51\xfb\xb1";

#[derive(Clone, Debug, Hash, PartialEq)]
pub struct Cryptor {
	key: Key<Aes256>,
}

impl Cryptor {
	pub fn new(access_key: impl AsRef<[u8]>, password: impl AsRef<[u8]>) -> Self {
		let mut md5 = Md5::new();
		md5.update(access_key);
		md5.update(KEY_SALT);
		let mut key = Vec::with_capacity(Md5::output_size() * 2);
		key.extend_from_slice(&md5.finalize());
		md5 = Md5::new();
		md5.update(KEY_SALT);
		md5.update(password);
		key.extend_from_slice(&md5.finalize());
		Self {
			key: key.as_slice().try_into().expect("MD5 result doesn't match key length"),
		}
	}

	pub fn decrypt(&self, data: impl AsRef<[u8]>) -> Result<String> {
		let mut data = BASE64_STANDARD.decode(&data)?;
		let decryptor = Decryptor::<Aes256>::new(&self.key);
		let out_len = decryptor
			.decrypt_padded::<ZeroPadding>(&mut data)
			.map_err(|e| CryptError(format!("Error during decryption: {e}")))?
			.len();
		data.truncate(out_len);
		Ok(String::from_utf8(data)?)
	}

	pub fn encrypt(&self, data: impl Into<Vec<u8>>) -> Result<String> {
		let mut data = data.into();
		let data_len = data.len();
		let block_size = Aes256::block_size();
		let enc_data_len = data_len + block_size - data_len % block_size;
		if enc_data_len > data.len() {
			data.extend(iter::repeat_n(0, enc_data_len - data.len()));
		}
		let encryptor = Encryptor::<Aes256>::new(&self.key);
		encryptor
			.encrypt_padded::<ZeroPadding>(&mut data, data_len)
			.map_err(|e| CryptError(format!("Error during encryption: {e}")))?;
		Ok(BASE64_STANDARD.encode(&data))
	}
}

#[cfg(test)]
mod tests {
	use super::Cryptor;

	#[test]
	fn encryption_preserves_protocol_bytes() {
		let cryptor = Cryptor::new("access-key", "password");
		assert_eq!(cryptor.encrypt(b"GET /system").unwrap(), "jN58ZL5M7wxW6/VWB8z/+g==");
		assert_eq!(
			cryptor.encrypt(b"0123456789abcdef").unwrap(),
			"AVTXwA1Cpo0bcEG2aufs4gAAAAAAAAAAAAAAAAAAAAA="
		);
	}

	#[test]
	fn decryption_removes_zero_padding_across_blocks() {
		let cryptor = Cryptor::new("access-key", "password");
		assert_eq!(cryptor.decrypt("jN58ZL5M7wxW6/VWB8z/+g==").unwrap(), "GET /system");
		// This 14-byte JSON body has 18 zero bytes of padding across two AES blocks.
		assert_eq!(
			cryptor.decrypt("bSQ2Cy05T5YvGtNCEhuUFknAgxJlyU+FnpHQlOnQzEI=").unwrap(),
			r#"{"value":true}"#
		);
	}
}
