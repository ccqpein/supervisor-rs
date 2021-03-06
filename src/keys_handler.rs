use openssl::pkey::{HasPrivate, HasPublic};
use openssl::rsa::{Padding, Rsa};
use std::fs::File;
use std::io::prelude::*;
use std::io::{Error, ErrorKind, Result};
use std::path::Path;

/// data wrapper including key_name, key_path, and decrypted data inside
#[derive(Debug, PartialEq)]
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
            key_name,
            key_path: Some(kpath.to_string()),
            data: data.to_string(),
        })
    }

    /// For server, receive data and parse to (keyname, encrypted data)
    pub fn unwrap_from(s: &[u8]) -> Result<(String, &[u8])> {
        // clean ";", which is 59
        let cache: Vec<&[u8]> = s.splitn(2, |num| *num == 59).collect();

        if cache.len() == 1 {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "Cannot parse encrypt data",
            ));
        }

        for c in &cache {
            if c.len() == 0 {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "Keyname or command is empty",
                ));
            }
        }

        let key_name = match String::from_utf8(cache[0].to_vec()) {
            Ok(s) => s,
            Err(e) => return Err(Error::new(ErrorKind::InvalidData, e.to_string())),
        };

        Ok((key_name, cache[1]))
    }

    /// decrypt
    pub fn decrypt_with_pubkey<T: HasPublic>(
        data: &[u8],
        keyname: String,
        pubkey: Rsa<T>,
    ) -> Result<Self> {
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
            data,
        })
    }

    /// encrypt
    fn encrypt_with_prikey<T: HasPrivate>(&self, prikey: Rsa<T>) -> Result<Vec<u8>> {
        let mut temp = vec![0; prikey.size() as usize];

        prikey.private_encrypt(self.data.as_bytes(), &mut temp, Padding::PKCS1)?;

        let mut result = self.key_name.as_bytes().to_vec();
        result.push(59);
        result.append(&mut temp);
        Ok(result)
    }

    /// keyname + ';' + encrypt data
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

        let (keyname, dd) = DataWrapper::unwrap_from(&a).unwrap();
        // after server recieve
        let b = DataWrapper::decrypt_with_pubkey(dd, keyname, rsa).unwrap();
        assert_eq!(data, b.data)
    }

    #[test] // test this under root directory of supervisor-rs
    fn read_form_real_keypairs_test() {
        let data = "foobar";

        // client side
        let client_side_data = DataWrapper::new("./test/pri.pem", data)
            .unwrap()
            .encrypt_to_bytes()
            .unwrap();

        // server side
        let (keyname, dd) = DataWrapper::unwrap_from(&client_side_data).unwrap();
        let mut pub_key = String::new();
        let _ = File::open("./test/pubkey/pub.pem")
            .unwrap()
            .read_to_string(&mut pub_key);
        let pub_key = Rsa::public_key_from_pem(&pub_key.as_bytes()).unwrap();
        let b = DataWrapper::decrypt_with_pubkey(dd, keyname, pub_key).unwrap();

        assert_eq!(b.data, data);
    }

    #[test]
    fn test_unwrap_from() {
        let failed_data = "aaaaabbbbbb".as_bytes();
        //dbg!(DataWrapper::unwrap_from(failed_data));
        assert!(DataWrapper::unwrap_from(failed_data).is_err());

        let failed_data1 = "aaaaa;bbbbbb".as_bytes();
        let dw1 = DataWrapper::unwrap_from(failed_data1).unwrap();
        assert_eq!(dw1, ("aaaaa".to_string(), "bbbbbb".as_bytes()));
    }
}
