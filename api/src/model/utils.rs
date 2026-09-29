use serde::Serialize;
use sqlx::prelude::FromRow;
use uuid::Uuid;

#[derive(FromRow)]
pub struct Id(Uuid);

#[derive(Serialize)]
pub struct PageInfo {
    pub page_size: usize,
    pub page_no: usize,
    pub total_rows: usize,
}

#[derive(Serialize)]
pub struct PaginatedList<T>
where
    T: Serialize,
{
    pub list: Vec<T>,
    pub page_info: PageInfo,
}
