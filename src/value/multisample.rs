use std::ops::{Not, Neg};
use num::{Zero, One};

use crate::{
    Env,
    value::{Value, ValueNode, CacheValue},
    effect::Delay,
};

#[derive(Copy, Clone, Default, Debug)]
pub struct MultiSample<T>(pub T, pub T);
impl<T: Zero + Default + Clone> Zero for MultiSample<T> {
    fn zero() -> Self {
        MultiSample(T::zero(), T::zero())
    }

    fn is_zero(&self) -> bool {
        self.0.is_zero() && self.1.is_zero()
    }
}
impl<T: One + Default + Clone> One for MultiSample<T> {
    fn one() -> Self {
        MultiSample(T::one(), T::one())
    }
}
impl<T: Clone> From<T> for MultiSample<T> {
    fn from(other: T) -> Self {
        MultiSample(other.clone(), other.clone())
    }
}
impl ValueNode for MultiSample<f64> {
    type T = MultiSample<f64>;
    fn fill_buffer(&mut self, _env: &Env, buffer: &mut [Self::T], samples: usize) {
        for i in 0..samples {
            buffer[i] = *self;
        }
    }
}

pub struct Bundler<'a, T>(pub Value<'a, T>, pub Value<'a, T>);


impl<'a, T: Zero + Default + Copy> ValueNode for Bundler<'a, T> {
    type T = MultiSample<T>;
    fn fill_buffer(&mut self, env: &Env, buffer: &mut [Self::T], samples: usize) {
        let mut a: Vec<T> = (0..samples).map(|_| T::zero()).collect();
        self.0.fill_buffer(env, &mut a, samples);
        let mut b: Vec<T> = (0..samples).map(|_| T::zero()).collect();
        self.1.fill_buffer(env, &mut b, samples);
        for (i, (a,b)) in a.iter().zip(&b).enumerate() {
            buffer[i] = MultiSample(*a, *b);
        }
    }
}


pub fn hass_shift<'a, T: Clone + Copy + Default + Zero + 'a>(input: impl Into<Value<'a, T>>, shift: f64) -> Value<'a, MultiSample<T>> {
    let input = CacheValue::new(input);
    let delayed = Delay::new(input.clone(), shift.abs());

    if shift > 0.0 {
        Bundler(input.into(), delayed.into())
    } else {
        Bundler(delayed.into(), input.into())
    }.into()
}


macro_rules! multisample_binary_operator {
    ( $operator_name:ident, $operator_assign_name:ident, $operator_method:ident, $operator_assign_method:ident, $operation:tt, $( $numeric:ident ),* ) =>  {
            #[allow(non_snake_case)]
            mod $operator_name {
                use std::ops::{$operator_name,};
                use super::MultiSample;

            impl<T: $operator_name<Output = T> + Default + Clone> $operator_name for MultiSample<T> {
                type Output = MultiSample<T>;
                #[inline]
                fn $operator_method(mut self, other: MultiSample<T>) -> Self::Output {
                    self.0 = self.0.clone() $operation other.0.clone();
                    self.1 = self.1.clone() $operation other.1.clone();
                    self
                }
            }

            $(
            impl<'a> $operator_name<MultiSample<$numeric>> for $numeric {
                type Output = MultiSample<$numeric>;

                #[inline]
                fn $operator_method(self, mut other: MultiSample<$numeric>) -> Self::Output {
                    other.0 = other.0 $operation self;
                    other.1 = other.1 $operation self;
                    other
                }
            }
            )*
        }
    }
}

multisample_binary_operator!(Add, AddAssign, add, add_assign, +, usize, u8, u16, u32, u64, u128, isize, i8, i16, i32, i64, i128, f32, f64);
multisample_binary_operator!(Sub, SubAssign, sub, sub_assign, -, usize, u8, u16, u32, u64, u128, isize, i8, i16, i32, i64, i128, f32, f64);
multisample_binary_operator!(Mul, MulAssign, mul, mul_assign, *, usize, u8, u16, u32, u64, u128, isize, i8, i16, i32, i64, i128, f32, f64);
multisample_binary_operator!(Div, DivAssign, div, div_assign, /, usize, u8, u16, u32, u64, u128, isize, i8, i16, i32, i64, i128, f32, f64);
multisample_binary_operator!(BitAnd, BitAndAssign, bitand, bitand_assign, &, bool, usize, u8, u16, u32, u64, u128, isize, i8, i16, i32, i64, i128);
multisample_binary_operator!(BitOr, BitOrAssign, bitor, bitor_assign, |, bool, usize, u8, u16, u32, u64, u128, isize, i8, i16, i32, i64, i128);
multisample_binary_operator!(BitXor, BitXorAssign, bitxor, bitxor_assign, ^, bool, usize, u8, u16, u32, u64, u128, isize, i8, i16, i32, i64, i128);
multisample_binary_operator!(Rem, RemAssign, rem, rem_assign, %, usize, u8, u16, u32, u64, u128, isize, i8, i16, i32, i64, i128, f32, f64);
multisample_binary_operator!(Shl, ShlAssign, shl, shl_assign, <<, usize, u8, u16, u32, u64, u128, isize, i8, i16, i32, i64, i128 );
multisample_binary_operator!(Shr, ShrAssign, shr, shr_assign, >>, usize, u8, u16, u32, u64, u128, isize, i8, i16, i32, i64, i128);   


impl<T: Default + Clone + Not<Output = T>> Not for MultiSample<T> {
    type Output = Self;

    fn not(mut self) -> Self::Output {
        self.0 = !self.0;
        self.1 = !self.1;
        self
    }
}
impl<T: Default + Clone + Neg<Output = T>> Neg for MultiSample<T> {
    type Output = Self;

    fn neg(mut self) -> Self::Output {
        self.0 = -self.0;
        self.1 = -self.1;
        self
    }
}
