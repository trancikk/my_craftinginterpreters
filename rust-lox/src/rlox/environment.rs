use crate::rlox::eval::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub struct Env {
    names: HashMap<String, Value>,
    parent_env: Option<Rc<RefCell<Env>>>,
}

pub type EnvRef = Rc<RefCell<Env>>;

impl Env {
    pub fn new() -> EnvRef {
        Rc::new(RefCell::new(Env {
            names: HashMap::new(),
            parent_env: None,
        }))
    }

    pub fn from_env(env: &EnvRef) -> EnvRef {
        Rc::new(RefCell::new(Env {
            names: HashMap::new(),
            parent_env: Some(Rc::clone(env)),
        }))
    }
    pub fn define(&mut self, name: String, value: Value) {
        self.names.insert(name, value);
    }

    pub fn get(&self, name: &str) -> Option<Value> {
        let cur_value = self.names.get(name);
        if cur_value.is_some() {
            cur_value.cloned()
        } else {
            match &self.parent_env {
                Some(env) => env.borrow().get(name),
                None => None,
            }
        }
    }

    pub fn assign(&mut self, name: &str, value: Value) -> bool {
        if self.names.contains_key(name) {
            self.names.insert(name.to_string(), value).is_some()
        } else if let Some(env) = &self.parent_env {
            env.borrow_mut().assign(name, value)
        } else {
            false
        }
    }
}
