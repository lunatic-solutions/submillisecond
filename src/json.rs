use http::{Request, Response};
use serde::{de, ser};

pub fn from_json<T>(req: Request<T>) -> serde_json::Result<Request<Vec<u8>>>
where
    T: ser::Serialize,
{
    let (parts, body) = req.into_parts();
    let body = serde_json::to_vec(&body)?;
    Ok(Request::from_parts(parts, body))
}

pub fn to_json<T>(res: Response<Vec<u8>>) -> serde_json::Result<Response<T>>
where
    for<'de> T: de::Deserialize<'de>,
{
    let (parts, body) = res.into_parts();
    let body = serde_json::from_slice(&body)?;
    Ok(Response::from_parts(parts, body))
}
