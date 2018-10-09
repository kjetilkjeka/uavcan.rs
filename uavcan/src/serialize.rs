
/// A `Cursor` used in partial (de)serialization of structures.
///
/// Defines a bit placement in the non-serialized structure.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Cursor {
    pub field: usize,
    pub bit: usize,
}

impl Cursor {
    /// Creates a new `Cursor` pointing to the beginning of a struct {field: 0, bit: 0}
    pub fn new() -> Self {
        Cursor {
            field: 0,
            bit: 0,
        }
    }
}
