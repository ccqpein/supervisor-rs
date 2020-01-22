use openssl::pkey::{HasPrivate, HasPublic};
use openssl::rsa::{Padding, Rsa};
use std::fs::File;
use std::io::prelude::*;
use std::io::{Error, ErrorKind, Result};
use std::path::Path;

#[derive(Debug)]
pub struct DataWrapper {
    key_name: String,
    key_path: Option<String>,
    pub data: String,
}

impl DataWrapper {
    pub fn new(kpath: &str, data: &str) -> Result<Self> {
        let key_name = if let Some(f) = Path::new(kpath).file_stem() {
            f.to_str().unwrap().to_string()
        } else {
            return Err(Error::new(
                ErrorKind::NotFound,
                "Key file path is not right, cannot get filename",
            ));
        };

        Ok(DataWrapper {
            key_name: key_name,
            key_path: Some(kpath.to_string()),
            data: data.to_string(),
        })
    }

    // for server, receive data and parse to (keyname, true data)
    pub fn unwrap_from(s: &[u8]) -> Result<(String, &[u8])> {
        // clean ";"
        let cache: Vec<&[u8]> = s.splitn(2, |num| *num == 59).collect();

        for c in &cache {
            if c.len() == 0 {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "parsed data should only have two parts",
                ));
            }
        }

        let key_name = match String::from_utf8(cache[0].to_vec()) {
            Ok(s) => s,
            Err(e) => return Err(Error::new(ErrorKind::InvalidData, e.to_string())),
        };

        Ok((key_name, cache[1]))
    }

    pub fn decrypt_with_pubkey<T: HasPublic>(s: &[u8], pubkey: Rsa<T>) -> Result<Self> {
        let (keyname, data) = Self::unwrap_from(s)?;

        // decrypt
        let mut temp = vec![0; pubkey.size() as usize];
        pubkey.public_decrypt(&data, &mut temp, Padding::PKCS1)?;

        temp.retain(|x| *x != 0);
        let data = match String::from_utf8(temp) {
            Ok(s) => s,
            Err(e) => return Err(Error::new(ErrorKind::InvalidData, e.to_string())),
        };

        Ok(Self {
            key_name: keyname,
            key_path: None,
            data: data,
        })
    }

    fn encrypt_with_prikey<T: HasPrivate>(&self, prikey: Rsa<T>) -> Result<Vec<u8>> {
        let mut temp = vec![0; prikey.size() as usize];

        prikey.private_encrypt(self.data.as_bytes(), &mut temp, Padding::PKCS1)?;

        let mut result = self.key_name.as_bytes().to_vec();
        result.push(59);
        result.append(&mut temp);
        Ok(result)
    }

    // keyname + ';' + encrypt data
    pub fn encrypt_to_bytes(&self) -> Result<Vec<u8>> {
        let path = match &self.key_path {
            Some(p) => p,
            None => return Err(Error::new(ErrorKind::NotFound, "no key file path input")),
        };

        let mut f = File::open(&path)?;
        let mut contents = String::new();
        f.read_to_string(&mut contents)?;

        let p_key = Rsa::private_key_from_pem(contents.as_bytes())?;

        self.encrypt_with_prikey(p_key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openssl::rsa::Rsa;

    #[test]
    fn work_flow_test() {
        let rsa = Rsa::generate(2048).unwrap();
        let data = "foobar";

        // data client send to server
        let a = DataWrapper::new("test", data)
            .unwrap()
            .encrypt_with_prikey(rsa.clone())
            .unwrap();

        // after server recieve
        let b = DataWrapper::decrypt_with_pubkey(&a, rsa).unwrap();
        assert_eq!(data, b.data)
    }
}
