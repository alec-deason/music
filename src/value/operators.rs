macro_rules! value_binary_operator {
    ( $operator_name:ident, $operator_method:ident, $operation:tt ) =>  {
        #[allow(non_snake_case)]
        mod $operator_name {
            use std::ops::$operator_name;
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

                fn $operator_method(self, other: Value<T>) -> Self::Output {
                    Operator {
                        a: self,
                        b: other,
                    }.into()
                }
            }
        }
    }
}

value_binary_operator!(Add, add, +);
value_binary_operator!(Sub, sub, -);
value_binary_operator!(Mul, mul, *);
value_binary_operator!(Div, div, /);
value_binary_operator!(BitAnd, bitand, &);
value_binary_operator!(BitOr, bitor, |);
value_binary_operator!(BitXor, bitxor, ^);
value_binary_operator!(Rem, rem, %);
value_binary_operator!(Shl, shl, <<);
value_binary_operator!(Shr, shr, >>);

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
