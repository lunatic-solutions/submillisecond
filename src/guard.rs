use crate::RequestContext;

/// Types which implement [`Guard`] can be used to protect routes.
///
/// This can be useful for admin-only routes for example.
///
/// Guards which return false will cause a 404 error if no other routes are
/// matched.
///
/// # Example
///
/// ```
/// use submillisecond::{router, Application, Guard, RequestContext};
///
/// struct AdminGuard;
///
/// impl Guard for AdminGuard {
///     fn check(&self, req: &RequestContext) -> bool {
///         is_admin(req)
///     }
/// }
///
/// router! {
///     "/admin" if AdminGuard => {
///         GET "/dashboard" => dashboard
///     }
/// }
/// ```
pub trait Guard {
    /// Checks a given request, returning a bool if the guard is valid.
    fn check(&self, req: &RequestContext) -> bool;
}
