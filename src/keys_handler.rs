use openssl::pkey::{HasPrivate, HasPublic};
use openssl::rsa::{Padding, Rsa};
use std::fs::File;
use std::io::prelude::*;
use std::io::{Error, ErrorKind, Result};

#[derive(Debug)]
pub struct DataWrapper {
    key_name: String,
    key_path: Option<String>,
    data: String,
}

impl DataWrapper {
    pub fn new(k: &str, data: &str) -> Self {
        DataWrapper {
            key_name: k.to_string(),
            key_path: None,
            data: data.to_string(),
        }
    }

    // for server
    fn unwrap_from(s: &[u8]) -> Result<(String, &[u8])> {
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

    fn decrypt_with_pubkey<T: HasPublic>(s: &[u8], pubkey: Rsa<T>) -> Result<Self> {
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

    //:= TODO: finish this function with server side
    // pub fn decrypt_from_bytes(s: &[u8]) -> Result<Self>{
    // }
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
            .encrypt_with_prikey(rsa.clone())
            .unwrap();

        // after server recieve
        let b = DataWrapper::decrypt_with_pubkey(&a, rsa).unwrap();
        assert_eq!(data, b.data)
    }
}
