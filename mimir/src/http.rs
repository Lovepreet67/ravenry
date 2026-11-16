use std::time::Instant;

use axum::{
    extract::{MatchedPath, Request},
    middleware::Next,
    response::Response,
};
use metrics::{counter, histogram};

use crate::{lables, names};

pub async fn track(req: Request, next: Next) -> Response {
    let route = req
        .extensions()
        .get::<MatchedPath>()
        .map(|p| p.as_str().to_owned())
        .unwrap_or_else(|| "unmatched".into());
    let start = Instant::now();
    let res = next.run(req).await;
    counter!(names::HTTP_REQUESTS,
        lables::ROUTE=>route.clone(),
        lables::STATUS_CLASS=>lables::status_class(res.status().as_u16())
    )
    .increment(1);
    histogram!(names::HTTP_DURATION, lables::ROUTE=>route).record(start.elapsed().as_secs_f64());
    res
}
