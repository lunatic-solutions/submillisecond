use crate::{extract::FromRequest, response::IntoResponse, Request, Response};

pub trait Handler {
    type Response: IntoResponse;

    fn handle(self, req: Request) -> Self::Response;
}

pub type FnPtr<Args, R, const ARRITY: usize> = <FnDecayer<ARRITY, Args> as FnPtrDecay<R>>::F;

pub trait Arity<Args> {
    const VALUE: usize;
}

pub const fn arity<A, T: Arity<A>>(_: &T) -> usize {
    T::VALUE
}

pub struct FnDecayer<const A: usize, Args>(core::marker::PhantomData<Args>);

pub trait FnPtrDecay<R> {
    type F;
}

macro_rules! impl_handler {
    ( $( $args: ident ),* ) => {
        impl<$( $args, )* R> Handler for fn( $( $args ),* ) -> R
        where
            $( $args: FromRequest, )*
            R: IntoResponse,
        {
            type Response = Response;

            #[allow(unused_mut, unused_variables)]
            fn handle(self, mut req: Request) -> Self::Response {
                paste::paste! {
                    $(
                        let [< $args:lower >] = match <$args as FromRequest>::from_request(&mut req) {
                            Ok(e) => e,
                            Err(err) => return err.into_response(),
                        };
                    )*
                    self( $( [< $args:lower >] ),* ).into_response()
                }
            }
        }
    };
}

impl_handler!();
all_the_tuples!(impl_handler);

macro_rules! impl_arity {
    ( $value: literal $( , $args: ident )* ) => {
        impl<Fnc, $( $args, )* R> Arity<($( $args, )*)> for Fnc
        where
            Fnc: Fn($( $args, )*) -> R,
        {
            const VALUE: usize = $value;
        }
    };
}

impl_arity!(0);
all_the_tuples!(impl_arity, numbered);

macro_rules! impl_fn_ptr_decay {
    ( $value: literal $( , $args: ident )* ) => {
        impl<$( $args, )* R> FnPtrDecay<R> for FnDecayer<$value, ($( $args, )*)> {
            type F = fn($( $args, )*) -> R;
        }
    };
}

impl_fn_ptr_decay!(0);
all_the_tuples!(impl_fn_ptr_decay, numbered);
