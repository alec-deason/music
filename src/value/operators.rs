macro_rules! value_binary_operator {
    ( $operator_name:ident, $operator_assign_name:ident, $operator_method:ident, $operator_assign_method:ident, $operation:tt, $( $numeric:ident ),* ) =>  {
        #[allow(non_snake_case)]
        mod $operator_name {
            use std::ops::{$operator_name, $operator_assign_name};
            use crate::{
                value::{ValueNode, Value},
                Env,
            };
            pub struct Operator<T> {
                a: Value<T>,
                b: Value<T>,
            }

            impl<T> ValueNode for Operator<T>
                where T: $operator_name<Output = T> + 'static {
                    type T = T;
                    fn next(&mut self, env: &Env) -> T {
                        let a = self.a.next(env);
                        let b = self.b.next(env);
                        a $operation b
                    }
            }

            impl<T: $operator_name<Output = T> + 'static> $operator_name<Value<T>> for Value<T> {
                type Output = Value<T>;

                #[inline]
                fn $operator_method(self, other: Value<T>) -> Self::Output {
                    Operator {
                        a: self,
                        b: other,
                    }.into()
                }
            }

            impl<T: $operator_name<Output = T> + 'static, D: ValueNode<T=T> + 'static> $operator_name<D> for Value<T> {
                type Output = Value<T>;

                #[inline]
                fn $operator_method(self, other: D) -> Self::Output {
                    Operator {
                        a: self,
                        b: other.into(),
                    }.into()
                }
            }

            $(
            impl $operator_name<Value<$numeric>> for $numeric {
                type Output = Value<$numeric>;

                #[inline]
                fn $operator_method(self, other: Value<$numeric>) -> Self::Output {
                    Operator {
                        a: self.into(),
                        b: other,
                    }.into()
                }
            }
            )*
        }
    }
}


//TODO: I take in idents for the assign versions of the operators but I couldn't figure out how to
//actually implement those traits so I'm not currently using them.
value_binary_operator!(Add, AddAssign, add, add_assign, +, usize, u8, u16, u32, u64, u128, isize, i8, i16, i32, i64, i128, f32, f64);
value_binary_operator!(Sub, SubAssign, sub, sub_assign, -, usize, u8, u16, u32, u64, u128, isize, i8, i16, i32, i64, i128, f32, f64);
value_binary_operator!(Mul, MulAssign, mul, mul_assign, *, usize, u8, u16, u32, u64, u128, isize, i8, i16, i32, i64, i128, f32, f64);
value_binary_operator!(Div, DivAssign, div, div_assign, /, usize, u8, u16, u32, u64, u128, isize, i8, i16, i32, i64, i128, f32, f64);
value_binary_operator!(BitAnd, BitAndAssign, bitand, bitand_assign, &, bool, usize, u8, u16, u32, u64, u128, isize, i8, i16, i32, i64, i128);
value_binary_operator!(BitOr, BitOrAssign, bitor, bitor_assign, |, bool, usize, u8, u16, u32, u64, u128, isize, i8, i16, i32, i64, i128);
value_binary_operator!(BitXor, BitXorAssign, bitxor, bitxor_assign, ^, bool, usize, u8, u16, u32, u64, u128, isize, i8, i16, i32, i64, i128);
value_binary_operator!(Rem, RemAssign, rem, rem_assign, %, usize, u8, u16, u32, u64, u128, isize, i8, i16, i32, i64, i128, f32, f64);
value_binary_operator!(Shl, ShlAssign, shl, shl_assign, <<, usize, u8, u16, u32, u64, u128, isize, i8, i16, i32, i64, i128 );
value_binary_operator!(Shr, ShrAssign, shr, shr_assign, >>, usize, u8, u16, u32, u64, u128, isize, i8, i16, i32, i64, i128);

//TODO: I should be able to do these with a similar macro to the binary operators but I was having
//trouble with types. It only saves a few lines of boilerplate anyway.
mod neg {
    use std::ops::Neg;
    use crate::{
        value::{ValueNode, Value},
        Env,
    };
    pub struct Operator<T> {
        v: Value<T>,
    }
    impl<T: Neg<Output = T>> ValueNode for Operator<T> {
            type T = T;
            fn next(&mut self, env: &Env) -> T {
                -self.v.next(env)
            }
    }
    impl<T: Neg<Output = T> + 'static> Neg for Value<T>
        where T: Neg<Output = T> {
        type Output = Value<T>;

        fn neg(self) -> Self::Output {
            Operator {
                v: self,
            }.into()
        }
    }
}

mod not {
    use std::ops::Not;
    use crate::{
        value::{ValueNode, Value},
        Env,
    };
    pub struct Operator<T> {
        v: Value<T>,
    }
    impl<T: Not<Output = T>> ValueNode for Operator<T> {
            type T = T;
            fn next(&mut self, env: &Env) -> T {
                !self.v.next(env)
            }
    }
    impl<T: Not<Output = T> + 'static> Not for Value<T>
        where T: Not<Output = T> {
        type Output = Operator<T>;

        fn not(self) -> Self::Output {
            Operator {
                v: self,
            }.into()
        }
    }
}
