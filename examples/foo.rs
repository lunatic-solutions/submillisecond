// #![no_implicit_prelude]

// macro_rules! router {
//     ($tt: tt*) => {};
// }

use submillisecond::router;

fn foo_handler() -> ::std::string::String {
    "fi".to_string()
}
// fn hi_handler() {}

// router! {
//     use my_middleware::foo;

//     GET "/foo" => foo_handler,
//     "/users" => {
//         GET "/hi" => hi_handler,
//     }
// }

// fn routar(
//     mut req: ::submillisecond::Request,
// ) -> ::std::result::Result<::submillisecond::Response, ::submillisecond::router::RouteError> {
//     const ROUTER: ::submillisecond_core::router::Router<'static, &'static str> =
//         ::submillisecond_core::router::Router::from_node(
//             ::submillisecond_core::router::tree::ConstNode {
//                 priority: 1,
//                 wild_child: true,
//                 indices: &[],
//                 node_type: ::submillisecond_core::router::tree::NodeType::Root,
//                 value: ::std::option::Option::Some("/foo"),
//                 prefix: b"",
//                 children: &[],
//             },
//         );

//     let path = req
//         .extensions()
//         .get::<::submillisecond::router::Route>()
//         .unwrap()
//         .path();
//     let route_match = ROUTER.at(path);
//     match route_match {
//         ::std::result::Result::Ok(::submillisecond_core::router::Match {
//             params,
//             value: route,
//         }) => {
//             if !params.is_empty() {
//                 match req
//                     .extensions_mut()
//                     .get_mut::<::submillisecond_core::router::params::Params>()
//                 {
//                     ::std::option::Option::Some(params_ext) => params_ext.merge(params),
//                     ::std::option::Option::None => {
//                         req.extensions_mut().insert(params);
//                     }
//                 }
//             }

//             match *route {
//                 "/foo" if req.method() == ::http::Method::GET => ::std::result::Result::Ok(
//                     ::submillisecond::response::IntoResponse::into_response(
//                         ::submillisecond::handler::Handler::handle(
//                             foo_handler
//                                 as ::submillisecond::handler::FnPtr<
//                                     _,
//                                     _,
//                                     { ::submillisecond::handler::arity(&foo_handler) },
//                                 >,
//                             req,
//                         ),
//                     ),
//                 ),
//                 _ => ::std::result::Result::Err(
//                     ::submillisecond::router::RouteError::RouteNotMatch(req),
//                 ),
//             }
//         }
//         ::std::result::Result::Err(_) => {
//             ::std::result::Result::Err(::submillisecond::router::RouteError::RouteNotMatch(req))
//         }
//     }
// }

fn main() {
    // routar();
    let router = router! {
        GET "/foo" if true => foo_handler
        GET "/bar" if true => foo_handler
        POST "/foo" => foo_handler
    };
}
