use lunatic::function::FuncRef;

use crate::extract::{FromOwnedRequest, FromRequest};
use crate::response::IntoResponse;
use crate::{RequestContext, Response};

/// A handler is implemented for any function which takes any number of
/// [extractors](crate::extract), and returns any type that implements
/// [`IntoResponse`].
///
/// To avoid unecessary clones, the [`RequestContext`], [`http::Request`],
/// [`String`], [`Vec<u8>`], [`Params`](crate::params::Params) extractors (and
/// any other types which implement [`FromOwnedRequest`] directly) should be
/// placed as the first argument, and cannot be used together in a single
/// handler.
///
/// A maximum of 16 extractor arguments may be added for a single handler.
///
/// # Handler examples
///
/// ```
/// fn index() -> &'static str {
///     "Hello, submillisecond"
/// }
///
/// use submillisecond::extract::Path;
/// use submillisecond::http::status::FOUND;
///
/// fn headers(Path(id): Path<String>) -> (StatusCode, String) {
///     (FOUND, id)
/// }
/// ```
///
/// # Middleware example
///
/// ```
/// use submillisecond::RequestContent;
/// use submillisecond::response::Response;
///
/// fn logging_layer(req: RequestContext) -> Response {
///     println!("Incoming request start");
///     let res = req.next_handler();
///     println!("Incoming request end");
///     res
/// }
/// ```
pub trait Handler<Arg = (), Ret = ()> {
    /// Handles the request, returning a response.
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

        #[allow(unused_parens)]
        impl<F, $arg1, $( $( $args, )*)? R> Handler<($arg1$(, $( $args, )*)?), R> for FuncRef<F>
        where
            F: Fn($arg1$(, $( $args, )*)?) -> R + lunatic::function::reference::Fn<F> + Copy,
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
                    self.get()(e1 $(, $( [< $args:lower >] ),*)?).into_response()
                }
            }
        }
    };
}

all_the_tuples!(impl_handler);
