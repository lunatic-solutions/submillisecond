use lunatic::function::reference::Fn as FnPtr;
use lunatic::function::FuncRef;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::extract::{FromOwnedRequest, FromRequest};
use crate::response::IntoResponse;
use crate::{RequestContext, Response};

/// Implemented for process-safe [`Handlers`](Handler).
///
/// Submillisecond handles every request in a separate [lunatic
/// process](lunatic::Process), and lunatic's processes are sandboxed. This
/// means that no memory is shared between the request handler and the rest of
/// the app. This introduces an additional limitation on what can be a
/// [`Handler`].
///
/// Two kinds of types are safe to be used as handlers:
/// - Static functions
/// - Serializable and clonable objects
///
/// ### Static functions
///
/// This type is obvious. Non-capturing functions are generated during compile
/// time and are shared between all processes, so they can be easily used as
/// handlers. In fact, the [`router!`](crate::router) macro will in the end just
/// generate a function that will be used as a handler and invoke other handlers
/// depending on the request values.
///
/// ### Serializable and clonable objects
///
/// Everything else needs to be passed somehow to the memory of the handler
/// process. This means that we need to clone the value for every incoming
/// request, serialize it and send it to the process handling the request.
pub trait ProcessSafeHandler<Kind, Arg, Ret> {
    /// A handler is only safe if it can be cloned and safely sent between
    /// processes.
    type SafeHandler: Handler<Arg, Ret> + Clone + Serialize + DeserializeOwned;

    /// Turn type into a safe handler.
    fn safe_handler(self) -> Self::SafeHandler;
}

/// Marker type for functions that satisfy [`ProcessSafeHandler`].
pub struct Function;
/// Marker type for objects that satisfy [`ProcessSafeHandler`].
pub struct Object;

impl<T, Arg, Ret> ProcessSafeHandler<Function, Arg, Ret> for T
where
    T: FnPtr<T> + Copy,
    FuncRef<T>: Handler<Arg, Ret>,
{
    type SafeHandler = FuncRef<T>;

    fn safe_handler(self) -> Self::SafeHandler {
        FuncRef::new(self)
    }
}

impl<T, Arg, Ret> ProcessSafeHandler<Object, Arg, Ret> for T
where
    T: Clone + Handler<Arg, Ret> + Serialize + DeserializeOwned,
{
    type SafeHandler = T;

    fn safe_handler(self) -> Self::SafeHandler {
        self
    }
}

impl<T, Arg, Ret> Handler<Arg, Ret> for FuncRef<T>
where
    T: FnPtr<T> + Copy + Handler<Arg, Ret>,
{
    fn handle(&self, req: RequestContext) -> Response {
        self.get().handle(req)
    }
}

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
    };
}

all_the_tuples!(impl_handler);
