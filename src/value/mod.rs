use std::rc::Rc;
use std::cell::RefCell;
use std::time::Duration;

mod operators;
pub use operators::*;

use super::Env;

pub trait ValueNode {
    type T;
    fn fill_buffer(&mut self, env: &Env, buffer: &mut [Self::T], samples: usize);
}

pub struct Value<'a, T>(Box<dyn ValueNode<T=T> + 'a>);
impl<'a, T: Default, D: ValueNode<T=T> + 'a> From<D> for Value<'a, T> {
    fn from(node: D) -> Self {
        Value(Box::new(node))
    }
}

impl<'a, T: Default> Value<'a, T> {
    pub fn fill_buffer(&mut self, env: &Env, buffer: &mut [T], samples: usize) {
        self.0.fill_buffer(env, buffer, samples);
    }
}

struct CacheValueState<T> {
    trigger: (Duration, usize),
    cached_value: Option<Vec<T>>,
}

pub struct CacheValue<'a, T> {
    value: Rc<RefCell<Value<'a, T>>>,
    state: Rc<RefCell<CacheValueState<T>>>
}

impl<'a, T> Clone for CacheValue<'a, T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            state: self.state.clone(),
        }
    }
}

impl<'a, T> CacheValue<'a, T> {
    pub fn new(value: impl Into<Value<'a, T>>) -> Self {
        CacheValue {
            value: Rc::new(RefCell::new(value.into())),
            state: Rc::new(RefCell::new(CacheValueState {
                trigger: (Duration::new(0, 0), 0),
                cached_value: None,
            })),
        }
    }
}


impl<'a, T: Clone + Default> ValueNode for CacheValue<'a, T> {
    type T = T;
    fn fill_buffer(&mut self, env: &Env, buffer: &mut [Self::T], samples: usize) {
        let mut state = self.state.borrow_mut();
        if state.cached_value.is_some() && state.trigger == (env.time, samples) {
            buffer[0..samples].clone_from_slice(state.cached_value.as_ref().unwrap());
        } else {
            self.value.borrow_mut().fill_buffer(env, buffer, samples);
            state.cached_value.replace(buffer[0..samples].to_vec());
            state.trigger = (env.time, samples);
        }
    }
}

macro_rules! value_node_impl_for_numerics {
    ( $($t:ident)* ) => ($(
        impl ValueNode for $t {
            type T = $t;
            fn fill_buffer(&mut self, _env: &Env, buffer: &mut [Self::T], samples: usize) {
                for i in 0..samples {
                    buffer[i] = *self;
                }
            }
        }
    )*)
}
value_node_impl_for_numerics! { usize u8 u16 u32 u64 u128 isize i8 i16 i32 i64 i128 f32 f64 bool }
