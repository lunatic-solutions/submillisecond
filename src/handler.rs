use crate::extract::{FromOwnedRequest, FromRequest};
use crate::response::IntoResponse;
use crate::{RequestContext, Response};

pub trait Handler<Arg = (), Ret = ()> {
    fn handle(&self, req: RequestContext) -> Response;
}

impl<F, R> Handler<(), R> for F
where
    F: Fn() -> R,
    R: IntoResponse,
{
    fn handle(&self, _req: RequestContext) -> Response {
        self().into_response()
    }
}

macro_rules! impl_handler {
    ( $arg1: ident $(, $( $args: ident ),*)? ) => {
        #[allow(unused_parens)]
        impl<F, $arg1, $( $( $args, )*)? R> Handler<($arg1$(, $( $args, )*)?), R> for F
        where
            F: Fn($arg1$(, $( $args, )*)?) -> R,
            $arg1: FromOwnedRequest,
            $( $( $args: FromRequest, )* )?
            R: IntoResponse,
        {

            #[allow(unused_mut, unused_variables)]
            fn handle(&self, mut req: RequestContext) -> Response {
                paste::paste! {
                    $($(
                        let [< $args:lower >] = match <$args as FromRequest>::from_request(&mut req) {
                            Ok(e) => e,
                            Err(err) => return err.into_response(),
                        };
                    )*)?
                    let e1 = match <$arg1 as FromOwnedRequest>::from_owned_request(req) {
                        Ok(e) => e,
                        Err(err) => return err.into_response(),
                    };
                    self(e1 $(, $( [< $args:lower >] ),*)?).into_response()
                }
            }
        }
    };
}

all_the_tuples!(impl_handler);
