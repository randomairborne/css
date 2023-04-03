mod assignment;
mod class;
mod info;
mod todo;
pub use assignment::*;
pub use class::*;
pub use info::*;
pub use todo::*;

#[derive(serde::Deserialize)]
pub struct PaginationQuery {
    page: Option<String>,
}