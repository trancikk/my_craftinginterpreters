use crate::rlox::eval::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub struct Env {
    names: HashMap<String, Value>,
    parent_env: Option<Rc<RefCell<Env>>>,
}

impl Env {
    pub fn new() -> Env {
        Env {
            names: HashMap::new(),
            parent_env: None,
        }
    }

    pub fn from_env(env: Rc<RefCell<Env>>) -> Env {
        Env {
            names: HashMap::new(),
            parent_env: Some(env),
        }
    }
    pub fn define(&mut self, name: String, value: Value) {
        self.names.insert(name.to_string(), value);
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        self.names.get(name)
    }

    pub fn assign(&mut self, name: &str, value: Value) -> bool {
        if self.names.contains_key(name) {
            self.names.insert(name.to_string(), value).is_some()
        } else {
            false
        }
    }
}
