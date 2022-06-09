use super::path::FailedToDeserializePathParams;

define_rejection! {
    #[status = INTERNAL_SERVER_ERROR]
    #[body = "No paths parameters found for matched route. Are you also extracting `Request<_>`?"]
    /// Rejection type used if axum's internal representation of path parameters
    /// is missing. This is commonly caused by extracting `Request<_>`. `Path`
    /// must be extracted first.
    pub struct MissingPathParams;
}

composite_rejection! {
    /// Rejection used for [`Path`](super::Path).
    ///
    /// Contains one variant for each way the [`Path`](super::Path) extractor
    /// can fail.
    pub enum PathRejection {
        FailedToDeserializePathParams,
        MissingPathParams,
    }
}
