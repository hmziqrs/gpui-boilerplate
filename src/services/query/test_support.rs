use super::*;

pub(super) fn resource() -> QueryResource<&'static str> {
    QueryResource::new(
        "demo",
        CachePolicy::Ttl { ttl_ms: 1_000 },
        RequestPolicy::LatestWins,
    )
}

pub(super) fn error_message<'a>(resource: &'a QueryResource<&'static str>) -> Option<&'a str> {
    resource.error().map(QueryError::message)
}
