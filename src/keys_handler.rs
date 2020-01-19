use openssl::pkey::{HasPrivate, HasPublic, Private, Public};
use openssl::rsa::{Padding, Rsa};
use std::io::{Error, ErrorKind, Result};

#[derive(Debug)]
struct DataWrapper {
    key_name: String,
    data: String,
}

impl DataWrapper {
    fn new(k: &str, data: &str) -> Self {
        DataWrapper {
            key_name: k.to_string(),
            data: data.to_string(),
        }
    }

    // for server
    fn unwrap_from(s: &[u8]) -> Result<(String, Vec<u8>)> {
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

        Ok((key_name, cache[1].to_vec()))
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
            data: data,
        })
    }

    fn encrypt_with_prikey<T: HasPrivate>(&self, prikey: Rsa<T>) -> Result<Vec<u8>> {
        let mut temp = vec![0; prikey.size() as usize];

        prikey.private_encrypt(self.data.as_bytes(), &mut temp, Padding::PKCS1)?;

        let mut result = self.key_name.as_bytes().to_vec();
        result.push(59);
        result.append(&mut temp);
        Ok(result.to_vec())
    }
}

//:= TODO: Need tests
#[cfg(test)]
mod tests {
    use super::*;
    use openssl::rsa::{Padding, Rsa};
    use std::str;

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
