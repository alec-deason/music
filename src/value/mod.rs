use std::rc::Rc;
use std::cell::RefCell;
use std::time::Duration;

mod operators;
pub use operators::*;

use super::Env;

pub trait ValueNode {
    type T;
    fn next(&mut self, env: &Env) -> Self::T;
    fn fill_buffer(&mut self, env: &mut Env, buffer: &mut [Self::T], offset: usize, samples: usize) {
        let one_sample = 1_000_000_000 / env.sample_rate;
        let one_sample = Duration::new(0, one_sample as u32);
        for i in 0..samples {
            buffer[i+offset] = self.next(&env);
            env.time += one_sample;
        }
    }
}

pub struct Value<T>(Box<ValueNode<T=T>>);
impl<T, D: ValueNode<T=T> + 'static> From<D> for Value<T> {
    fn from(node: D) -> Self {
        Value(Box::new(node))
    }
}

impl<T> Value<T> {
    pub fn next(&mut self, env: &Env) -> T {
        self.0.next(env)
    }
    pub fn fill_buffer(&mut self, env: &mut Env, buffer: &mut [T], offset: usize, samples: usize) {
        self.0.fill_buffer(env, buffer, offset, samples);
    }
}

struct CacheValueState<T> {
    trigger: Duration,
    cached_value: Option<T>,
}

pub struct CacheValue<T> {
    value: Rc<RefCell<Value<T>>>,
    state: Rc<RefCell<CacheValueState<T>>>
}

impl<T> Clone for CacheValue<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            state: self.state.clone(),
        }
    }
}

impl<T> CacheValue<T> {
    pub fn new(value: impl Into<Value<T>>) -> Self {
        CacheValue {
            value: Rc::new(RefCell::new(value.into())),
            state: Rc::new(RefCell::new(CacheValueState {
                trigger: Duration::new(0, 0),
                cached_value: None,
            })),
        }
    }
}


impl<T: Clone> ValueNode for CacheValue<T> {
    type T = T;
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
}

macro_rules! value_node_impl_for_numerics {
    ( $($t:ident)* ) => ($(
        impl ValueNode for $t {
            type T = $t;
            fn next(&mut self, _env: &Env) -> Self::T {
                *self
            }
        }
    )*)
}
value_node_impl_for_numerics! { usize u8 u16 u32 u64 u128 isize i8 i16 i32 i64 i128 f32 f64 bool }
