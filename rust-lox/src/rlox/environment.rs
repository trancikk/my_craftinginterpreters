use crate::rlox::eval::Value;
use crate::rlox::parser::Literal::True;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub struct Env {
    pub(in crate::rlox::environment) names: HashMap<String, Value>,
    pub(in crate::rlox::environment) parent_env: Option<Rc<RefCell<Env>>>,
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
    pub fn define(env: &EnvRef, name: String, value: Value) {
        env.borrow_mut().names.insert(name, value);
    }

    pub fn resolve(env_ref: &EnvRef, name: &str) -> Option<EnvRef> {
        let parent = {
            let env = env_ref.borrow();

            if env.names.contains_key(name) {
                return Some(env_ref.clone());
            }
            env.parent_env.clone()
        };
        parent.and_then(|parent| Self::resolve(&parent, name))
    }

    pub fn assign_local(env_ref: &EnvRef, name: &str, value: Value) {
        env_ref.borrow_mut().names.insert(name.to_string(), value);
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
}
