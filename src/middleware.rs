use crate::{extract::FromRequest, IntoResponse, Request, Response};

/// Convenience trait alias for `FnOnce(Request) -> Response`.
pub trait Next: FnOnce(Request) -> Response {
    fn next(self, req: Request) -> Response;
}

impl<T: FnOnce(Request) -> Response> Next for T {
    fn next(self, req: Request) -> Response {
        self(req)
    }
}

pub trait Middleware<N: Next, Arg = (), Ret = Response> {
    fn apply(this: Self, req: Request, next: N) -> Response;
}

macro_rules! impl_middleware {
    ( $( $args: ident ),* ) => {
        impl<N, F, $( $args, )* R> Middleware<N, ($( $args, )*), R> for F
        where
            N: Next,
            F: FnOnce(Request, N, $( $args, )*) -> R,
            $( $args: FromRequest, )*
            R: IntoResponse,
        {

            #[allow(unused_mut, unused_variables)]
            fn apply(this: Self, mut req: Request, next: N) -> Response {
                paste::paste! {
                    $(
                        let [< $args:lower >] = match <$args as FromRequest>::from_request(&mut req) {
                            Ok(e) => e,
                            Err(err) => return err.into_response(),
                        };
                    )*
                    this(req, next, $( [< $args:lower >] ),* ).into_response()
                }
            }
        }
    };
}

impl_middleware!();
all_the_tuples!(impl_middleware);
