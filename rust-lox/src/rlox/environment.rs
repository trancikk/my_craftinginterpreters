use crate::rlox::eval::Value;
use crate::rlox::var_resolver::Resolution;
use crate::rlox::var_resolver::Resolution::{Global, Local, Unresolved};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

#[derive(Debug)]
pub struct Env {
    pub(self) names: HashMap<String, Value>,
    pub(self) parent_env: Option<EnvRef>,
    pub(self) global: WeakEnvRef,
}

pub type EnvRef = Rc<RefCell<Env>>;
pub type WeakEnvRef = Weak<RefCell<Env>>;

impl Env {
    pub fn new_global() -> EnvRef {
        Rc::new_cyclic(|this| {
            RefCell::new(Env {
                names: HashMap::new(),
                parent_env: None,
                global: this.clone(),
            })
        })
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

    pub fn resolve_env(env_ref: &EnvRef, name: &str, at: &Resolution) -> Option<EnvRef> {
        match at {
            Local(at) => Self::get_env_at(env_ref, at.to_owned()),
            Global => env_ref.borrow().global.upgrade(),
            Unresolved => Self::get_recursively(env_ref, name),
        }
    }

    // pub fn resolve_at(env_ref: &EnvRef, name: &str, at: u16) -> Option<EnvRef> {
    //     if at == 0 {
    //         return Some(env_ref.clone());
    //     }
    //     Self::get_recursively(env_ref, name)
    //         .and_then(|parent| Self::resolve_at(&parent, name, at - 1))
    // }

    fn get_recursively(env_ref: &EnvRef, name: &str) -> Option<EnvRef> {
        let env = env_ref.borrow();

        if env.names.contains_key(name) {
            return Some(env_ref.clone());
        }
        if let Some(parent_env) = env.parent_env.clone() {
            Self::get_recursively(&parent_env, name)
        } else {
            None
        }
    }

    fn get_env_at(env_ref: &EnvRef, at: u16) -> Option<EnvRef> {
        if at > 0 {
            let mut parent = Some(env_ref.clone());
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
        let target = Self::get_recursively(env_ref, name);
        if let Some(env) = target {
            env.borrow().names.get(name).cloned()
        } else {
            None
        }
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

    pub fn get_at(env_ref: &EnvRef, name: &str, at: &Resolution) -> Option<Value> {
        Self::resolve_env(env_ref, name, at).and_then(|env| env.borrow().names.get(name).cloned())
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

    pub fn assign_at(env: &EnvRef, name: &str, value: Value, at: &Resolution) -> bool {
        let target_env = Self::resolve_env(env, name, at);
        if let Some(env) = target_env {
            env.borrow_mut().assign_local(name, value);
            true
        } else {
            false
        }
    }
}
