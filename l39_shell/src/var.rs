use std::{collections::HashMap, str::FromStr};

#[derive(Clone, Debug)]
pub struct Var(pub String, pub String);

impl FromStr for Var {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (key, value) = s.split_once('=').unwrap_or((s, ""));

        if key.is_empty() {
            anyhow::bail!("export: not valid in this context");
        }

        Ok(Self(key.to_string(), value.to_string()))
    }
}

#[derive(Default)]
pub struct Vars {
    vars: HashMap<String, String>,
}

impl Vars {
    pub fn append(&mut self, vars: impl IntoIterator<Item = Var>) {
        for Var(key, value) in vars {
            if let Some(cur_val) = self.vars.get_mut(&key) {
                *cur_val = value;
            } else {
                self.vars.insert(key, value);
            }
        }
    }

    pub fn remove(&mut self, vars: impl IntoIterator<Item = Var>) {
        for Var(key, _) in vars {
            self.vars.remove(&key);
        }
    }
}
