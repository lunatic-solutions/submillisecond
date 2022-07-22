use crate::{extract::FromRequest, response::IntoResponse, Request, Response};

pub trait Handler<Arg = (), Ret = ()> {
    fn handle(this: Self, req: Request) -> Response;
}

impl<F, R> Handler<(), R> for F
where
    F: FnOnce() -> R,
    R: IntoResponse,
{
    fn handle(this: Self, _req: Request) -> Response {
        this().into_response()
    }
}

macro_rules! impl_handler {
    ( $arg1: ident $(, $( $args: ident ),*)? ) => {
        #[allow(unused_parens)]
        impl<F, $arg1, $( $( $args, )*)? R> Handler<($arg1$(, $( $args, )*)?), R> for F
        where
            F: FnOnce($arg1$(, $( $args, )*)?) -> R,
            $arg1: FromRequest,
            $( $( $args: FromRequest, )* )?
            R: IntoResponse,
        {

            #[allow(unused_mut, unused_variables)]
            fn handle(this: Self, mut req: Request) -> Response {
                paste::paste! {
                    $($(
                        let [< $args:lower >] = match <$args as FromRequest>::from_request(&mut req) {
                            Ok(e) => e,
                            Err(err) => return err.into_response(),
                        };
                    )*)?
                    let e1 = match <$arg1 as FromRequest>::from_owned_request(req) {
                        Ok(e) => e,
                        Err(err) => return err.into_response(),
                    };
                    this(e1$(, $( [< $args:lower >] ),*)?).into_response()
                }
            }
        }
    };
}

all_the_tuples!(impl_handler);
