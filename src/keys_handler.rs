use openssl::pkey::{Private, Public};
use openssl::rsa::{Padding, Rsa};
use std::io::{Error, ErrorKind, Result};

#[derive(Debug)]
struct DataWrapper {
    key_name: String,
    encrypted_data: String,
}

impl DataWrapper {
    fn unwrap_from(s: &str) -> Result<Self> {
        let cache: Vec<&str> = s.split(";; ").collect();

        if cache.len() != 2 {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "parsed data should only have two parts",
            ));
        }

        Ok(Self {
            key_name: cache[0].to_string(),
            encrypted_data: cache[1].to_string(),
        })
    }

    fn wrap_from(keyname: String, prikey: Rsa<Private>, s: &str) -> Result<Self> {
        Ok(Self {
            key_name: keyname,
            encrypted_data: Self::encrypt_with_prikey(s, prikey)?,
        })
    }

    fn decrypt_with_pubkey(&self, pubkey: Rsa<Public>) -> Result<String> {
        let mut temp = vec![0; pubkey.size() as usize];

        pubkey.public_decrypt(self.encrypted_data.as_bytes(), &mut temp, Padding::PKCS1)?;

        match String::from_utf8(temp) {
            Ok(r) => Ok(r),
            Err(e) => Err(Error::new(ErrorKind::InvalidData, e)),
        }
    }

    fn encrypt_with_prikey(s: &str, prikey: Rsa<Private>) -> Result<String> {
        let mut temp = vec![0; prikey.size() as usize];

        prikey.private_encrypt(s.as_bytes(), &mut temp, Padding::PKCS1)?;

        match String::from_utf8(temp) {
            Ok(r) => Ok(r),
            Err(e) => Err(Error::new(ErrorKind::InvalidData, e)),
        }
    }
}

//:= TODO: Need tests
