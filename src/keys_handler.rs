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

    fn wrap_from(keyname: String, s: &str) -> Result<Self> {
        Ok(Self {
            key_name: keyname,
            encrypted_data: s.to_string(),
        })
    }

    fn decrypt(&self,pubkey) -> Result<String> {
        
    }
}
