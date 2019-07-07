macro_rules! value_binary_operator {
    ( $operator_name:ident, $operator_method:ident, $operation:expr ) =>  {
        #[allow(non_snake_case)]
        mod $operator_name {
            use std::ops::$operator_name;
            use crate::{
                value::{Value, ValueNode},
                Env,
            };
            pub struct Operator<T> {
                a: Value<T>,
                b: Value<T>,
            }
            impl<T> ValueNode<T> for Operator<T>
                where T: $operator_name<Output = T> + 'static {
                    fn next(&mut self, env: &Env) -> T {
                        let a = self.a.next(env);
                        let b = self.b.next(env);
                        $operation(a, b)
                    }

                    fn to_value(self) -> Value<T> {
                        Value(Box::new(self))
                    }
            }
            impl<T> $operator_name for Value<T>
                where T: $operator_name<Output = T> + 'static {
                type Output = Value<T>;

                fn $operator_method(self, other: Value<T>) -> Self::Output {
                    Value(Box::new(Operator {
                        a: self,
                        b: other,
                    }))
                }
            }
        }
    }
}

value_binary_operator!(Add, add, |a, b| a+b);
value_binary_operator!(Sub, sub, |a, b| a-b);
value_binary_operator!(Mul, mul, |a, b| a*b);
value_binary_operator!(Div, div, |a, b| a/b);
value_binary_operator!(BitAnd, bitand, |a, b| a&b);
value_binary_operator!(BitOr, bitor, |a, b| a|b);
value_binary_operator!(BitXor, bitxor, |a, b| a^b);
value_binary_operator!(Rem, rem, |a, b| a%b);
value_binary_operator!(Shl, shl, |a, b| a<<b);
value_binary_operator!(Shr, shr, |a, b| a>>b);

//TODO: I should be able to do these with a similar macro to the binary operators but I was having
//trouble with types. It only saves a few lines of boilerplate anyway.
mod neg {
    use std::ops::Neg;
    use crate::{
        value::{Value, ValueNode},
        Env,
    };
    pub struct Operator<T> {
        v: Value<T>,
    }
    impl<T> ValueNode<T> for Operator<T>
        where T: Neg<Output = T> + 'static{
            fn next(&mut self, env: &Env) -> T {
                -self.v.next(env)
            }

            fn to_value(self) -> Value<T> {
                Value(Box::new(self))
            }
    }
    impl<T> Neg for Value<T>
        where T: Neg<Output = T> + 'static {
        type Output = Value<T>;

        fn neg(self) -> Self::Output {
            Value(Box::new(Operator {
                v: self,
            }))
        }
    }
}

mod not {
    use std::ops::Not;
    use crate::{
        value::{Value, ValueNode},
        Env,
    };
    pub struct Operator<T> {
        v: Value<T>,
    }
    impl<T> ValueNode<T> for Operator<T>
        where T: Not<Output = T> + 'static {
            fn next(&mut self, env: &Env) -> T {
                !self.v.next(env)
            }

            fn to_value(self) -> Value<T> {
                Value(Box::new(self))
            }
    }
    impl<T> Not for Value<T>
        where T: Not<Output = T> + 'static {
        type Output = Value<T>;

        fn not(self) -> Self::Output {
            Value(Box::new(Operator {
                v: self,
            }))
        }
    }
}
