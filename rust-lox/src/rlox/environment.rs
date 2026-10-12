use crate::rlox::eval::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug, PartialEq)]
pub struct Env {
    pub(self) names: HashMap<String, Value>,
    pub(self) parent_env: Option<EnvRef>,
    pub(self) global: Option<EnvRef>,
}

pub type EnvRef = Rc<RefCell<Env>>;

impl Env {
    pub fn new() -> EnvRef {
        Rc::new(RefCell::new(Env {
            names: HashMap::new(),
            parent_env: None,
            global: None,
        }))
    }

    pub fn from_env(env: &EnvRef) -> EnvRef {
        Rc::new(RefCell::new(Env {
            names: HashMap::new(),
            parent_env: Some(Rc::clone(env)),
            global: env.borrow().global.clone(),
        }))
    }
    pub fn define(env: &EnvRef, name: String, value: Value) {
        env.borrow_mut().names.insert(name, value);
    }

    pub fn resolve(env_ref: &EnvRef, name: &str) -> Option<EnvRef> {
        Self::get_from_parent(env_ref, name).and_then(|parent| Self::resolve(&parent, name))
    }

    pub fn resolve_at(env_ref: &EnvRef, name: &str, at: u16) -> Option<EnvRef> {
        if at == 0 {
            return Some(env_ref.clone());
        }
        Self::get_from_parent(env_ref, name)
            .and_then(|parent| Self::resolve_at(&parent, name, at - 1))
    }

    fn get_from_parent(env_ref: &EnvRef, name: &str) -> Option<EnvRef> {
        let env = env_ref.borrow();

        if env.names.contains_key(name) {
            return Some(env_ref.clone());
        }
        env.parent_env.clone()
    }

    fn get_parent_at(env_ref: &EnvRef, at: u16) -> Option<EnvRef> {
        if at > 0 {
            let mut parent = env_ref.borrow().parent_env.clone();
            for _ in 0..at {
                parent = parent.and_then(|parent| parent.borrow().parent_env.clone())
            }
            parent
        } else {
            Some(env_ref.clone())
        }
    }

    pub fn assign_local(&mut self, name: &str, value: Value) {
        self.names.insert(name.to_string(), value);
    }

    pub fn get(env_ref: &EnvRef, name: &str) -> Option<Value> {
        let parent = {
            let env = env_ref.borrow();

            if let Some(value) = env.names.get(name) {
                return Some(value.clone());
            }
            env.parent_env.clone()
        };
        parent.and_then(|parent| Self::get(&parent, name))
        // let env = env.borrow();
        // let cur_value = env.names.get(name);
        // if cur_value.is_some() {
        //     cur_value.cloned()
        // } else {
        //     match env.parent_env {
        //         Some(env) => Env::get(&env, name),
        //         None => None,
        //     }
        // }
    }

    pub fn assign(env_ref: &EnvRef, name: &str, value: Value) -> bool {
        let parent = {
            let mut env = env_ref.borrow_mut();

            if let Some(current_value) = env.names.get_mut(name) {
                *current_value = value;
                return true;
            }
            env.parent_env.clone()
        };

        match parent {
            Some(parent) => Self::assign(&parent, name, value),
            None => false,
        }
    }

    pub fn assign_at(env: &EnvRef, name: &str, value: Value, at: Option<u16>) {
        let target_env = if let Some(at) = at {
            Self::get_parent_at(env, at)
        } else {
            env.borrow().global.clone()
        };
        target_env.unwrap().borrow_mut().assign_local(name, value);
    }
}
