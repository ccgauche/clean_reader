use super::Row;

/// A fully lowered `<table>`. Rows are ordered top-to-bottom as they
/// appeared in the source document.
#[derive(Debug)]
pub struct Table<'a> {
    pub rows: Vec<Row<'a>>,
}
