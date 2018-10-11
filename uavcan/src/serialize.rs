use Serializable;


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

/// This represents serializable types where all fields are statically known.
///
/// Should generally be derived by using `#[derive(UavcanStruct)]`.
pub trait SerializableStatic: Serializable {
    /// Number of primitive fields after flattening of data type.
    ///
    /// Flattening of a struct consists of replacing all structs with its fields.
    /// Flattening of an enum consists of putting all fields in order
    ///
    /// # Examples
    /// ## Flattening of struct
    /// ```
    /// # #[macro_use]
    /// # extern crate uavcan;
    /// # use uavcan::Struct;
    /// # use uavcan::Serializable;
    /// # use uavcan::SerializableStatic;
    /// #[derive(UavcanStruct)]
    /// struct InnerStruct {
    ///     v1: u8,
    ///     v2: u8,
    /// }
    ///
    /// #[derive(UavcanStruct)]
    /// struct OuterStruct {
    ///     v1: InnerStruct,
    ///     v2: InnerStruct,
    /// }
    ///
    /// # fn main() {
    /// assert_eq!(InnerStruct::FLATTENED_FIELDS_NUMBER, 2);
    /// assert_eq!(OuterStruct::FLATTENED_FIELDS_NUMBER, 4);
    /// # }
    /// ```
    /// ## Flattening of enum
    /// ```
    /// # #[macro_use]
    /// # extern crate uavcan;
    /// # use uavcan::Struct;
    /// # use uavcan::Serializable;
    /// # use uavcan::SerializableStatic;
    /// #[derive(UavcanStruct)]
    /// enum InnerEnum {
    ///     V1(u8),
    ///     V2(u8),
    /// }
    ///
    /// #[derive(UavcanStruct)]
    /// enum OuterEnum {
    ///     V1(InnerEnum),
    ///     V2(InnerEnum),
    /// }
    ///
    /// # fn main() {
    /// assert_eq!(InnerEnum::FLATTENED_FIELDS_NUMBER, 2);
    /// assert_eq!(OuterEnum::FLATTENED_FIELDS_NUMBER, 4);
    /// # }
    /// ```
    const FLATTENED_FIELDS_NUMBER: usize;
}