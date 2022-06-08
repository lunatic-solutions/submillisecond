use std::ops;

use serde::de::DeserializeOwned;

use crate::Request;

use super::FromRequest;

#[derive(Debug)]
pub struct Path<T>(pub T);

impl<T> ops::Deref for Path<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> ops::DerefMut for Path<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> FromRequest for Path<T>
where
    T: DeserializeOwned,
{
    type Rejection = (); // PathRejection;

    fn from_request(req: &mut Request) -> Result<Self, Self::Rejection> {
        todo!();
    }
}
