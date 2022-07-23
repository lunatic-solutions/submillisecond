use std::ops::Deref;

use serde::de::DeserializeOwned;

use crate::RequestContext;

use super::{
    rejection::{FailedToDeserializeQueryString, QueryRejection},
    FromRequest,
};

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
