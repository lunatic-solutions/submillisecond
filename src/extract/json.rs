use std::fmt::Debug;

use serde::de::DeserializeOwned;

use crate::{
    json::{json_content_type, Json},
    Request,
};

use super::{
    rejection::{JsonDataError, JsonRejection, JsonSyntaxError, MissingJsonContentType},
    FromRequest,
};

impl<T> FromRequest for Json<T>
where
    T: DeserializeOwned + Debug,
{
    type Rejection = JsonRejection;

    fn from_request(req: &mut Request) -> Result<Self, Self::Rejection> {
        if !json_content_type(req) {
            return Err(MissingJsonContentType.into());
        }

        let body = String::from_utf8_lossy(req.body());
        println!("{body}");
        let pp = serde_json::from_str::<T>(&body);
        println!("{pp:?}");
        println!("{:?}", req.body());

        let value = match serde_json::from_slice(req.body()) {
            Ok(value) => value,
            Err(err) => {
                let rejection = match err.classify() {
                    serde_json::error::Category::Data => JsonDataError::from_err(err).into(),
                    serde_json::error::Category::Syntax | serde_json::error::Category::Eof => {
                        JsonSyntaxError::from_err(err).into()
                    }
                    serde_json::error::Category::Io => {
                        if cfg!(debug_assertions) {
                            // we don't use `serde_json::from_reader` and instead always buffer
                            // bodies first, so we shouldn't encounter any IO errors
                            unreachable!()
                        } else {
                            JsonSyntaxError::from_err(err).into()
                        }
                    }
                };
                return Err(rejection);
            }
        };

        Ok(Json(value))
    }
}
