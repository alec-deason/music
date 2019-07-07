use std::rc::Rc;
use std::cell::RefCell;
use std::time::Duration;

mod operators;
pub use operators::*;

use super::Env;

pub struct Value<T>(pub Box<ValueNode<T> + 'static>);
pub trait ValueNode<T> {
    fn next(&mut self, env: &Env) -> T;
    fn fill_buffer(&mut self, env: &Env, buffer: &mut [T], offset: usize, samples: usize) {
        let mut env = env.clone();
        let one_sample = 1_000_000_000 / env.sample_rate;
        let one_sample = Duration::new(0, one_sample as u32);
        for i in 0..samples {
            buffer[i+offset] = self.next(&env);
            env.time += one_sample;
        }
    }
    fn to_value(self) -> Value<T>;
}

#[derive(Clone)]
struct CacheValueState<T> {
    trigger: Duration,
    cached_value: Option<T>,
}

#[derive(Clone)]
pub struct CacheValue<T> {
    value: Rc<RefCell<Value<T>>>,
    state: Rc<RefCell<CacheValueState<T>>>
}

impl<T> CacheValue<T> {
    pub fn new(value: Value<T>) -> Self {
        CacheValue {
            value: Rc::new(RefCell::new(value)),
            state: Rc::new(RefCell::new(CacheValueState {
                trigger: Duration::new(0, 0),
                cached_value: None,
            })),
        }
    }
}


use std::fmt::Display;
impl<T: 'static> ValueNode<T> for CacheValue<T> where T: Clone + Display {
    fn next(&mut self, env: &Env) -> T {
        let mut state = self.state.borrow_mut();
        if (state.cached_value.is_none()) || (state.trigger != env.time) {
            let v = self.value.borrow_mut().next(env);
            state.cached_value.replace(v.clone());
            state.trigger = env.time;
            v
        } else {
            let v = state.cached_value.as_ref().unwrap().clone();
            v
        }
    }

    fn to_value(self) -> Value<T> {
        Value(Box::new(self))
    }
}

impl<T> From<T> for Value<T>
    where T: ValueNode<T> + 'static {
    fn from(src: T) -> Value<T> {
        Value(Box::new(src))
    }
}

impl<T: 'static> ValueNode<T> for Value<T> {
    fn next(&mut self, env: &Env) -> T {
        self.0.next(env)
    }

    fn to_value(self) -> Value<T> {
        Value(Box::new(self))
    }
}

impl ValueNode<f32> for f32 {
    fn next(&mut self, _env: &Env) -> f32 {
        *self
    }

    fn to_value(self) -> Value<f32> {
        Value(Box::new(self))
    }
}

impl ValueNode<f64> for f64 {
    fn next(&mut self, _env: &Env) -> f64 {
        *self
    }

    fn to_value(self) -> Value<f64> {
        Value(Box::new(self))
    }
}
