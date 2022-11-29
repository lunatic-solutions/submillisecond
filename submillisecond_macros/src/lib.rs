mod named_param;
mod router;
mod static_router;

use proc_macro::TokenStream;
use router::Router;
use static_router::StaticRouter;
use syn::{parse_macro_input, DeriveInput};

/// The `NamedParam` derive macro can be used to implement `FromRequest` for a
/// struct.
///
/// If using with unnamed struct, then the `#[param(name = "...")]` attribute
/// should be used.
///
/// If using with a struct with named fields, then each field name should match
/// the ones defined in the router.
///
/// # Struct with fields example
///
/// ```ignore
/// #[derive(NamedParam)]
/// struct Params {
///     name: String,
///     age: i32,
/// }
///
/// fn user_name_age(Params { name, age }: Params) -> String {
///     format!("Hello {name}, you are {age} years old")
/// }
///
/// router! {
///     GET "/user/:name/:age" => user_name_age
/// }
/// ```
///
/// # Unnamed struct example
///
/// ```ignore
/// #[derive(NamedParam)]
/// #[param(name = "age")]
/// struct AgeParam(i32);
///
/// fn age_param(AgeParam(age): AgeParam) -> String {
///     format!("You are {age} years old")
/// }
///
/// router! {
///     GET "/user/:age" => age_param
/// }
/// ```
#[proc_macro_derive(NamedParam, attributes(param))]
pub fn named_param(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match named_param::NamedParam::try_from(input) {
        Ok(named_param) => named_param.expand().into(),
        Err(err) => err.into_compile_error().into(),
    }
}

/// Macro for defining a router in [submillisecond](https://github.com/lunatic-solutions/submillisecond).
///
/// The syntax in this macro is aimed to be as simple and intuitive as possible.
///
/// # Handlers
///
/// Handlers are routes with a HTTP method, path and handler.
///
/// A basic example would be:
///
/// ```ignore
/// router! {
///     GET "/home" => home_handler
/// }
/// ```
///
/// In this example, `home_handler` is a function we defined which implements
/// `Handler`.
///
/// Multiple routes can be defined with handlers:
///
/// ```ignore
/// router! {
///     GET "/" => index_handler
///     GET "/about" => about_handler
///     POST "/avatar" => avatar_handler
/// }
/// ```
///
/// ## Methods
///
/// The supported methods are:
///
/// - GET
/// - POST
/// - PUT
/// - DELETE
/// - HEAD
/// - OPTIONS
/// - PATCH
///
/// # Sub-routers
///
/// Routers can be nested to create more complex routing.
///
/// Sub-routers are a similar syntax as handlers, except that they do not have a
/// method prefix, and have curly braces after the `=>`.
///
/// ```ignore
/// router! {
///     "/admin" => {
///         GET "/dashboard" => admin_dashboard
///         POST "/auth" => admin_auth
///     }
/// }
/// ```
///
/// The syntax in-between `{` and `}` is the same as the `router` macro itself.
///
/// # Layers/middleware
///
/// Handlers which call [`submillisecond::RequestContext::next_handler`](https://docs.rs/submillisecond/latest/submillisecond/struct.RequestContext.html#method.next_handler) are
/// considered to be middleware.
///
/// Middleware can be used in the router macro using the `with` keyword.
///
/// ```ignore
/// router! {
///     with global_logger;
/// }
/// ```
///
/// Multiple middleware can be used with the array syntax.
///
/// ```ignore
/// router! {
///     with [layer_one, layer_two];
/// }
/// ```
///
/// In the examples above, the middleware is used on the whole router.
/// Instead, we can also use middleware on a per-route basis.
///
/// ```ignore
/// router! {
///     GET "/" with logger_layer => index_handler
/// }
/// ```
///
/// When using guards, middleware should be placed after the if statement.
///
/// ```ignore
/// router! {
///     GET "/admin" if IsAdmin with logger_layer => admin_handler
/// }
/// ```
///
/// # Guards
///
/// Guards can be used to protect routes.
///
/// The syntax is similar to a regular `if` statement, and is placed after the
/// path of a route.
///
/// ```ignore
/// router! {
///     GET "/admin" if IsAdmin => admin_handler
/// }
/// ```
///
/// # Syntax
///
/// ##### RouterDefinition
///
/// > `{`
/// >
/// > &nbsp;&nbsp;&nbsp;&nbsp;[_RouterMiddleware_]﹖ `;`
/// >
/// > &nbsp;&nbsp;&nbsp;&nbsp;[_RouterItem_]*
/// >
/// > &nbsp;&nbsp;&nbsp;&nbsp;[_RouterCatchAll_]﹖
/// >
/// > `}`
///
/// ##### RouterItem
///
/// > [_RouterMethod_]﹖ [STRING_LITERAL] [_RouterIfStmt_]﹖
/// > [_RouterMiddleware_] `=>` [_RouterItemValue_]
///
/// ##### RouterItemValue
///
/// > [IDENTIFIER] | [_RouterDefinition_]
///
/// ##### RouterMethod
///
/// > `GET` | `POST` | `PUT` | `DELETE` | `HEAD` | `OPTIONS` | `PATCH`
///
/// ##### RouterMiddleware
///
/// > `with` [_RouterMiddlewareItem_]
///
/// ##### RouterMiddlewareItem
///
/// > [IDENTIFIER] | `[` [IDENTIFIER] `]`
///
/// ##### RouterIfStmt
///
/// > `if` [Expression]
///
/// ##### RouterCatchAll
///
/// > `_` `=>` [_RouterItemValue_]
///
/// [_RouterDefinition_]: #routerdefinition
/// [_RouterMiddleware_]: #routermiddleware
/// [_RouterMiddlewareItem_]: #routermiddlewareitem
/// [_RouterItem_]: #routeritem
/// [_RouterItemValue_]: #routeritemvalue
/// [_RouterMethod_]: #routermethod
/// [_RouterIfStmt_]: #routerifstmt
/// [_RouterCatchAll_]: #routercatchall
///
/// [IDENTIFIER]: https://doc.rust-lang.org/reference/identifiers.html
/// [STRING_LITERAL]: https://doc.rust-lang.org/reference/tokens.html#string-literals
/// [Expression]: https://doc.rust-lang.org/reference/expressions.html
#[proc_macro]
pub fn router(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as Router);
    input.expand().into()
}

/// The static router can be used to serve static files within a folder.
///
/// Two arguments can be passed to the router, with the first one being
/// optional:
/// - The path to your static directory (relative to your Cargo.toml).
/// - A custom 404 handler (optional).
///
/// # Basic example
///
/// ```ignore
/// static_router!("./static")
/// ```
///
/// # Example with custom 404 handler
///
/// ```ignore
/// static_router!("./static", handle_404)
/// ```
#[proc_macro]
pub fn static_router(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as StaticRouter);
    input.expand().into()
}

macro_rules! hquote {( $($tt:tt)* ) => (
    ::quote::quote_spanned! { ::proc_macro2::Span::mixed_site()=>
        $($tt)*
    }
)}
pub(crate) use hquote;
