use std::collections::HashMap;
use std::io::{Error as ioError, ErrorKind, Result};

use yaml_rust::Yaml;

#[derive(Debug)]
pub struct Hooks {
    hook_table: HashMap<String, String>,
}

//hooks:
//  - prehook: start child
//  - posthook: start child2

impl Hooks {
    pub fn new(input: &Yaml) -> Result<Self> {
        let hooks = match input.as_vec() {
            Some(hooks) => hooks,
            None => {
                return Err(ioError::new(
                    ErrorKind::InvalidData,
                    format!("hook format wrong"),
                ));
            }
        };

        let mut result = Self {
            hook_table: HashMap::new(),
        };

        for hook in hooks {
            if let Some(entrys) = hook.as_hash() {
                for entry in entrys {
                    if let Some(key) = entry.0.as_str().clone() {
                        if let Some(v) = entry.1.as_str().clone() {
                            result.hook_table.insert(key.to_string(), v.to_string());
                        }
                    }
                }
            }
        }

        Ok(result)
    }

    pub fn has_hook(&self) -> bool {
        self.hook_table.len() != 0
    }

    pub fn get(&self, key: &String) -> Option<&String> {
        self.hook_table.get(key)
    }

    pub fn get_hook_detail(&self, key: &String) -> Option<Vec<String>> {
        if let Some(hook_comm) = self.get(key) {
            return Some(
                hook_comm
                    .split_whitespace()
                    .map(|x| x.to_string())
                    .collect::<Vec<String>>(),
            );
        }
        None
    }

    //:= TODO: need test
    pub fn get_hook_command(&self, key: &String) -> Option<String> {
        if let Some(hook) = self.get_hook_detail(key) {
            return Some(hook[0].clone());
        }
        None
    }
}

impl Clone for Hooks {
    fn clone(&self) -> Self {
        Self {
            hook_table: self.hook_table.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Hooks;
    //use yaml_rust::Yaml;
    use yaml_rust::YamlLoader;

    #[test]
    fn test_parse_hook() {
        let test0 = YamlLoader::load_from_str(
            "
hooks:
  - prehook: start child
  - posthook: start child2
",
        )
        .unwrap();

        println!("{:#?}", Hooks::new(&test0[0]["hooks"]));

        let test1 = YamlLoader::load_from_str(
            "
hook:
  - prehook: start child1
  - prehook: start child2
",
        )
        .unwrap();

        println!("{:#?}", Hooks::new(&test1[0]["hooks"]));

        let test2 = YamlLoader::load_from_str(
            "
hooks:
  - prehook: start child1
  - prehook: start child2
",
        )
        .unwrap();

        println!("{:#?}", Hooks::new(&test2[0]["hooks"]));

        let test3 = YamlLoader::load_from_str(
            "
test: a
",
        )
        .unwrap();

        println!("{:#?}", Hooks::new(&test3[0]["hooks"]));
    }
}
