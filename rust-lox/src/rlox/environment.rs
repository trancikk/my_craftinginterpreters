use crate::rlox::eval::Value;
use std::collections::HashMap;

pub struct Env {
    names: HashMap<String, Value>,
}

impl Env {
    pub fn new() -> Env {
        Env {
            names: HashMap::new(),
        }
    }
    pub fn define(&mut self, name: String, value: Value) {
        self.names.insert(name.to_string(), value);
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        self.names.get(name)
    }
}
