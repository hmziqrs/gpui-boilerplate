use crate::core::*;

pub(crate) fn resource() -> QueryResource<&'static str> {
    QueryResource::new(
        "demo",
        CachePolicy::Ttl { ttl_ms: 1_000 },
        RequestPolicy::LatestWins,
    )
}

pub(crate) fn error_message<'a>(resource: &'a QueryResource<&'static str>) -> Option<&'a str> {
    resource.error().map(QueryError::message)
}
