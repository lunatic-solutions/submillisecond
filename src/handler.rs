use crate::{
    extract::FromRequest, params::Params, request_context, response::IntoResponse,
    router::RouteError, uri_reader::UriReader, Request, Response,
};

pub type HandlerFn<Req = Vec<u8>, Res = Vec<u8>> =
    fn(Request<Req>, Params, UriReader) -> Result<Response<Res>, RouteError>;

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
                request_context::run_before(&mut req);
                paste::paste! {
                    $(
                        let [< $args:lower >] = match <$args as FromRequest>::from_request(&mut req) {
                            Ok(e) => e,
                            Err(err) => return err.into_response(),
                        };
                    )*
                    let mut __resp = self( $( [< $args:lower >] ),* ).into_response();
                    request_context::drain(&mut __resp);
                    __resp
                }
            }
        }
    };
}

impl_handler!();
impl_handler!(E1);
impl_handler!(E1, E2);
impl_handler!(E1, E2, E3);
impl_handler!(E1, E2, E3, E4);
impl_handler!(E1, E2, E3, E4, E5);
impl_handler!(E1, E2, E3, E4, E5, E6);
impl_handler!(E1, E2, E3, E4, E5, E6, E7);
impl_handler!(E1, E2, E3, E4, E5, E6, E7, E8);
impl_handler!(E1, E2, E3, E4, E5, E6, E7, E8, E9);
impl_handler!(E1, E2, E3, E4, E5, E6, E7, E8, E9, E10);
impl_handler!(E1, E2, E3, E4, E5, E6, E7, E8, E9, E10, E11);
impl_handler!(E1, E2, E3, E4, E5, E6, E7, E8, E9, E10, E11, E12);
impl_handler!(E1, E2, E3, E4, E5, E6, E7, E8, E9, E10, E11, E12, E13);
impl_handler!(E1, E2, E3, E4, E5, E6, E7, E8, E9, E10, E11, E12, E13, E14);
impl_handler!(E1, E2, E3, E4, E5, E6, E7, E8, E9, E10, E11, E12, E13, E14, E15);
impl_handler!(E1, E2, E3, E4, E5, E6, E7, E8, E9, E10, E11, E12, E13, E14, E15, E16);

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
impl_arity!(1, A);
impl_arity!(2, A, B);
impl_arity!(3, A, B, C);
impl_arity!(4, A, B, C, D);
impl_arity!(5, A, B, C, D, E);
impl_arity!(6, A, B, C, D, E, F);
impl_arity!(7, A, B, C, D, E, F, G);
impl_arity!(8, A, B, C, D, E, F, G, H);
impl_arity!(9, A, B, C, D, E, F, G, H, I);
impl_arity!(10, A, B, C, D, E, F, G, H, I, J);
impl_arity!(11, A, B, C, D, E, F, G, H, I, J, K);
impl_arity!(12, A, B, C, D, E, F, G, H, I, J, K, L);
impl_arity!(13, A, B, C, D, E, F, G, H, I, J, K, L, M);
impl_arity!(14, A, B, C, D, E, F, G, H, I, J, K, L, M, N);
impl_arity!(15, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
impl_arity!(16, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);

macro_rules! impl_fn_ptr_decay {
    ( $value: literal $( , $args: ident )* ) => {
        impl<$( $args, )* R> FnPtrDecay<R> for FnDecayer<$value, ($( $args, )*)> {
            type F = fn($( $args, )*) -> R;
        }
    };
}

impl_fn_ptr_decay!(0);
impl_fn_ptr_decay!(1, A);
impl_fn_ptr_decay!(2, A, B);
impl_fn_ptr_decay!(3, A, B, C);
impl_fn_ptr_decay!(4, A, B, C, D);
impl_fn_ptr_decay!(5, A, B, C, D, E);
impl_fn_ptr_decay!(6, A, B, C, D, E, F);
impl_fn_ptr_decay!(7, A, B, C, D, E, F, G);
impl_fn_ptr_decay!(8, A, B, C, D, E, F, G, H);
impl_fn_ptr_decay!(9, A, B, C, D, E, F, G, H, I);
impl_fn_ptr_decay!(10, A, B, C, D, E, F, G, H, I, J);
impl_fn_ptr_decay!(11, A, B, C, D, E, F, G, H, I, J, K);
impl_fn_ptr_decay!(12, A, B, C, D, E, F, G, H, I, J, K, L);
impl_fn_ptr_decay!(13, A, B, C, D, E, F, G, H, I, J, K, L, M);
impl_fn_ptr_decay!(14, A, B, C, D, E, F, G, H, I, J, K, L, M, N);
impl_fn_ptr_decay!(15, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
impl_fn_ptr_decay!(16, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);
