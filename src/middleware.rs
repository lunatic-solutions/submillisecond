use crate::{extract::FromRequest, IntoResponse, Request, Response, RouteError};

/// Convenience trait alias for `FnOnce(Request) -> Result<Response, RouteError>`.
pub trait Next: FnOnce(Request) -> Result<Response, RouteError> {
    fn next(self, req: Request) -> Result<Response, RouteError>;
}

impl<T: FnOnce(Request) -> Result<Response, RouteError>> Next for T {
    fn next(self, req: Request) -> Result<Response, RouteError> {
        self(req)
    }
}

pub trait Middleware<N: Next, Arg = (), Ret = ()> {
    fn apply(this: Self, req: Request, next: N) -> Result<Response, RouteError>;
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
            fn apply(this: Self, mut req: Request, next: N) -> Result<Response, RouteError> {
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
