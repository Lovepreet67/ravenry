pub const PIPELINE: &str = "pipline";
pub const OUTCOME: &str = "outcome";
pub const RESULT: &str = "result";
pub const STATUS_CLASS: &str = "status_class";
pub const ROUTE: &str = "route";
pub const GROUP: &str = "group";
pub const WORKER_TYPE: &str = "worker_type";

pub fn status_class(code: u16) -> &'static str {
    match code {
        100..=199 => "1xx",
        200..=299 => "2xx",
        300..=399 => "3xx",
        400..=499 => "4xx",
        _ => "5xx",
    }
}
