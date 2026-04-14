use super::TableCell;

/// A single table row — an ordered list of header-or-data cells.
///
/// Exists so the table type hierarchy reads top-to-bottom: `Table ⇒
/// Row ⇒ TableCell`. The previous representation was
/// `Vec<Vec<(bool, TextCompound)>>`, which forced every reader to
/// reverse-engineer the shape.
#[derive(Debug)]
pub struct Row<'a> {
    pub cells: Vec<TableCell<'a>>,
}
