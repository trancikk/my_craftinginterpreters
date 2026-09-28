use crate::rlox::environment::{Env, EnvRef};
use crate::rlox::eval::Value::Number;
use crate::rlox::eval::{Callable, RuntimeError, Value};
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug)]
struct ClockFunction;

impl Callable for ClockFunction {
    fn call(&self, _args: Vec<Value>) -> Result<Value, RuntimeError> {
        let time = SystemTime::now();
        let duration_since_epoch = time.duration_since(UNIX_EPOCH).unwrap();
        let milliseconds_timestamp = duration_since_epoch.as_millis();
        Ok(Number(milliseconds_timestamp as f64))
    }

    fn get_name(&self) -> String {
        String::from("clock")
    }

    fn get_arity(&self) -> u8 {
        0
    }
}
pub fn define_clock(env_ref: EnvRef) {
    Env::define(
        &env_ref,
        String::from("clock"),
        Value::CallableValue(Rc::new(ClockFunction {})),
    );
}
