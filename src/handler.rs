use crate::{extract::FromRequest, response::IntoResponse, Request, Response};

pub trait Handler<Arg = (), Ret = ()> {
    fn handle(this: Self, req: Request) -> Result<Response, RouteError>;
}

macro_rules! impl_handler {
    ( $( $args: ident ),* ) => {
        impl<F, $( $args, )* R> Handler<($( $args, )*), R> for F
        where
            F: FnOnce($( $args, )*) -> R,
            $( $args: FromRequest, )*
            R: IntoResponse,
        {

            #[allow(unused_mut, unused_variables)]
            fn handle(this: Self, mut req: Request) -> Result<Response, RouteError> {
                paste::paste! {
                    $(
                        let [< $args:lower >] = match <$args as FromRequest>::from_request(&mut req) {
                            Ok(e) => e,
                            Err(err) => return err.into_response(),
                        };
                    )*
                    this( $( [< $args:lower >] ),* ).into_response()
                }
            }
        }
    };
}

impl_handler!();
all_the_tuples!(impl_handler);

#[derive(Debug)]
pub enum RouteError {
    ExtractorError(Response),
    RouteNotMatch(Request),
}

impl IntoResponse for RouteError {
    fn into_response(self) -> Result<Response, RouteError> {
        Err(self)
    }
}
