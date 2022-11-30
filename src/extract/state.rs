use serde::{Deserialize, Serialize};

use super::rejection::{NotInitialized, StateRejection};
use super::FromRequest;
use crate::state::State;
use crate::{Error, RequestContext};

impl<T> FromRequest for State<T>
where
    T: Clone + Serialize + for<'de> Deserialize<'de>,
{
    type Rejection = StateRejection;

    fn from_request(_req: &mut RequestContext) -> Result<Self, Self::Rejection> {
        State::<T>::load().ok_or_else(|| {
            StateRejection::NotInitialized(NotInitialized::from_err(Error::new(format!(
                "state should be initialized with State::<{}>::init(state)",
                std::any::type_name::<T>()
            ))))
        })
    }
}
