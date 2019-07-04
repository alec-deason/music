mod operators;
pub use operators::*;

use super::Env;

pub struct Value<'a, T>(pub Box<ValueNode<T> + 'a>);
pub trait ValueNode<T> {
    fn next(&mut self, env: &Env) -> T;
    fn fill_buffer(&mut self, env: &Env, buffer: &mut [T], offset: usize, samples: usize) {
        for i in 0..samples {
            buffer[i+offset] = self.next(env);
        }
    }
}

impl<'a, T> From<T> for Value<'a, T>
    where T: ValueNode<T> + 'a {
    fn from(src: T) -> Value<'a, T> {
        Value(Box::new(src))
    }
}

impl<'a, T> ValueNode<T> for Value<'a, T> {
    fn next(&mut self, env: &Env) -> T {
        self.0.next(env)
    }
}

impl ValueNode<f32> for f32 {
    fn next(&mut self, _env: &Env) -> f32 {
        *self
    }
}

impl ValueNode<f64> for f64 {
    fn next(&mut self, _env: &Env) -> f64 {
        *self
    }
}
