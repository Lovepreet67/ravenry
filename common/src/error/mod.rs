pub enum error {
    ConnectionError(String),
}

pub type CoreResult<T> = Result<T, error>;
