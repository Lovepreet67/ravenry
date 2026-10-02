use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;
use uuid::Uuid;

#[derive(FromRow)]
pub struct Id(Uuid);

#[derive(Debug, Deserialize)]
pub struct Pagination {
    page_no: Option<i64>,
    page_size: Option<i64>,
}
impl Pagination {
    pub fn get_page_no(&self) -> i64 {
        self.page_no.unwrap_or(1)
    }
    pub fn get_page_size(&self) -> i64 {
        self.page_size.unwrap_or(25).clamp(1, 100)
    }
}

#[derive(Serialize)]
pub struct PageInfo {
    pub page_size: i64,
    pub page_no: i64,
    pub total_rows: i64,
}

#[derive(Serialize)]
pub struct PaginatedList<T>
where
    T: Serialize,
{
    pub list: Vec<T>,
    pub page_info: PageInfo,
}
