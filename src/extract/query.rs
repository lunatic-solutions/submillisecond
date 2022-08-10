use std::ops::Deref;

use serde::de::DeserializeOwned;

use super::rejection::{FailedToDeserializeQueryString, QueryRejection};
use super::FromRequest;
use crate::RequestContext;

/// Extractor that deserializes query strings into some type.
///
/// `T` is expected to implement [`serde::Deserialize`].
///
/// # Example
///
/// ```rust,no_run
/// use submillisecond::{router, extract::Query};
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct Pagination {
///     page: usize,
///     per_page: usize,
/// }
///
/// // This will parse query strings like `?page=2&per_page=30` into `Pagination`
/// // structs.
/// fn list_things(pagination: Query<Pagination>) {
///     let pagination: Pagination = pagination.0;
///
///     // ...
/// }
///
/// router! {
///     GET "/list_things" => list_things
/// }
/// ```
///
/// If the query string cannot be parsed it will reject the request with a `422
/// Unprocessable Entity` response.
#[derive(Debug, Clone, Copy, Default)]
pub struct Query<T>(pub T);

impl<T> FromRequest for Query<T>
where
    T: DeserializeOwned,
{
    type Rejection = QueryRejection;

    fn from_request(req: &mut RequestContext) -> Result<Self, Self::Rejection> {
        let query = req.uri().query().unwrap_or_default();
        let value = serde_urlencoded::from_str(query)
            .map_err(FailedToDeserializeQueryString::__private_new::<T, _>)?;
        Ok(Query(value))
    }
}

impl<T> Deref for Query<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
