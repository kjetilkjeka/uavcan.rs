pub use ux::{
    u2, u3, u4, u5, u6, u7, u9, u10, u11, u12, u13, u14, u15, u17, u18, u19, u20, u21, u22, u23, u24, u25, u26, u27, u28, u29, u30, u31,
    u33, u34, u35, u36, u37, u38, u39, u40, u41, u42, u43, u44, u45, u46, u47, u48, u49, u50, u51, u52, u53, u54, u55, u56, u57, u58, u59, u60, u61, u62, u63,
    i2, i3, i4, i5, i6, i7, i9, i10, i11, i12, i13, i14, i15, i17, i18, i19, i20, i21, i22, i23, i24, i25, i26, i27, i28, i29, i30, i31,
    i33, i34, i35, i36, i37, i38, i39, i40, i41, i42, i43, i44, i45, i46, i47, i48, i49, i50, i51, i52, i53, i54, i55, i56, i57, i58, i59, i60, i61, i62, i63,
};
pub use half::f16;

use lib;
use lib::core::fmt;
use lib::core::cmp;
use lib::core::ops::{
    Index,
    IndexMut,
};

use bit_field::BitField;

use {
    Serializable,
    SerializableStatic,
    Cursor,
};

use serializer::{
    SerializationResult,
    SerializationBuffer,
};

use deserializer::{
    DeserializationResult,
    DeserializationBuffer,
};

trait PrimitiveType : Sized + Copy + ::Serializable {
    /// Mask bits exceeding `BIT_LENGTH`
    fn from_bits(v: u64) -> Self;
    
    /// Zeroes bits exceeding `BIT_LENGTH`
    fn to_bits(self) -> u64;
}
    

/// The Uavcan dynamic array type
///
/// # Examples
/// ```
/// use std::str;
/// use uavcan::types::*;
///
/// let dynamic_array = Dynamic::<[u8; 90]>::with_data("dynamic array".as_bytes());
///
/// assert_eq!(dynamic_array.length(), 13);
/// assert_eq!(str::from_utf8(dynamic_array.as_ref()).unwrap(), "dynamic array");
///
/// ```
pub struct Dynamic<T> {
    array: lib::core::mem::ManuallyDrop<T>,
    current_length: usize,
    deserialized_length: usize,
}

macro_rules! impl_array{
    {[$(($size:expr, $length_bits:expr)), *]} => {$(impl_array!(($size, $length_bits));)*};
    {($size:expr, $length_bits:expr)} => {

        // first implement static arrays
        impl<T: SerializableStatic> SerializableStatic for [T; $size] {
            const FLATTENED_FIELDS_NUMBER: usize = $size * T::FLATTENED_FIELDS_NUMBER;
        }

        impl<T: SerializableStatic> Serializable for [T; $size] {
            fn partial_serialize(&self, cursor: &mut Cursor, buffer: &mut SerializationBuffer) -> SerializationResult {
                while cursor.field < Self::FLATTENED_FIELDS_NUMBER {
                    let element = cursor.field  / T::FLATTENED_FIELDS_NUMBER;
                    cursor.field -= element*T::FLATTENED_FIELDS_NUMBER;
                    match self[element].partial_serialize(cursor, buffer) {
                        SerializationResult::Finished => {
                            cursor.field += element*T::FLATTENED_FIELDS_NUMBER;
                        },
                        SerializationResult::BufferFull => {
                            cursor.field += element*T::FLATTENED_FIELDS_NUMBER;
                            return SerializationResult::BufferFull;
                        },
                    }
                }
                
                cursor.field = Self::FLATTENED_FIELDS_NUMBER;
                cursor.bit = 0;
                SerializationResult::Finished
            }
            
            fn partial_deserialize(&mut self, cursor: &mut Cursor, buffer: &mut DeserializationBuffer) -> DeserializationResult {
                while cursor.field < Self::FLATTENED_FIELDS_NUMBER {
                    let element = cursor.field / T::FLATTENED_FIELDS_NUMBER;
                    cursor.field -= element*T::FLATTENED_FIELDS_NUMBER;
                    match self[element].partial_deserialize(cursor, buffer) {
                        DeserializationResult::Finished => {
                            cursor.field += element*T::FLATTENED_FIELDS_NUMBER;
                        },
                        DeserializationResult::BufferInsufficient => {
                            cursor.field += element*T::FLATTENED_FIELDS_NUMBER;
                            return DeserializationResult::BufferInsufficient;
                        },
                    }
                }
                
                cursor.field = Self::FLATTENED_FIELDS_NUMBER;
                cursor.bit = 0;
                DeserializationResult::Finished
            }
        }


        
        impl<T> Dynamic<[T; $size]> {
            pub const LENGTH_BITS: usize = $length_bits;
            pub const MAX_LENGTH: usize = $size;

            /// Constructs a new empty `Dynamic` array
            pub fn new() -> Self {
                Self{
                    array: lib::core::mem::ManuallyDrop::new(unsafe{ lib::core::mem::uninitialized() }),
                    current_length: 0,
                    deserialized_length: 0,
                }
            }
            
            /// Constructs a new `Dynamic` array with cloned data
            pub fn with_data(data: &[T]) -> Self where T: Clone{
                let mut s = Self::new();
                for i in 0..data.len() {
                    unsafe{lib::core::ptr::write(&mut s.array[i] as *mut T, data[i].clone())};
                }
                s.current_length = data.len();
                s
            }

            /// Push an item to the end of the `Dynamic` array. Size will increase by one after this operation.
            pub fn push(&mut self, item: T) {
                assert!(self.current_length < Self::MAX_LENGTH, "Can't push data to full array");
                unsafe{lib::core::ptr::write(&mut self.array[self.current_length] as *mut T, item)};
                self.current_length += 1;                
            }

            /// Returns the current length for the dynamic array
            pub fn length(&self) -> usize {
                self.current_length
            }

            /// Set lengths of the array.
            ///
            /// 
            /// When array is shrinked, the elements that fall out of range is dropped.
            /// When array is grown, `Default::default()` is inserted for the new values.
            pub fn set_length(&mut self, length: usize) where T: Default {
                if length < self.current_length {
                    self.shrink(length);
                } else if length > self.current_length {
                    self.grow(length);
                }
            }

            /// Shrinks array, dropping elements that fall out of range
            pub fn shrink(&mut self, length: usize) {
                assert!(length <= self.current_length, "Dynamic::shrink() can only be used to shrink array");
                for i in length..self.current_length {
                    let temp: T = lib::core::mem::replace(&mut self.array[i], unsafe{ lib::core::mem::uninitialized() } );
                    drop(temp);
                }
                self.current_length = length;
            }

            /// Grow array, inserting the default element in the new spaces
            fn grow(&mut self, length: usize) where T: Default {
                assert!(length > self.current_length);
                for i in self.current_length..length {
                    unsafe{lib::core::ptr::write(&mut self.array[i], T::default())};
                }
                self.current_length = length;
            }

            pub fn iter(&self) -> lib::core::slice::Iter<T> {
                self.array[0..self.current_length].iter()
            }
            
            pub fn iter_mut(&mut self) -> lib::core::slice::IterMut<T> {
                self.array[0..self.current_length].iter_mut()
            }

        }

        impl<T: SerializableStatic> SerializableStatic for Dynamic<[T; $size]> {
            const FLATTENED_FIELDS_NUMBER: usize = $size * T::FLATTENED_FIELDS_NUMBER + 1;
        }

        impl<T: SerializableStatic> Serializable for Dynamic<[T; $size]> {
            fn partial_serialize(&self, cursor: &mut Cursor, buffer: &mut SerializationBuffer) -> SerializationResult {

                let buffer_bits_remaining = buffer.bits_remaining();

                if buffer_bits_remaining == 0 {
                    return SerializationResult::BufferFull;
                }
                
                if cursor.field == 0 {
                    
                    let type_bits_remaining = Self::LENGTH_BITS - cursor.bit;
                    
                    if buffer_bits_remaining >= type_bits_remaining {
                        buffer.push_bits(type_bits_remaining, self.current_length.get_bits((cursor.bit as u8)..(Self::LENGTH_BITS as u8)) as u64);
                        cursor.field = 1;
                        cursor.bit = 0;
                    } else {
                        buffer.push_bits(buffer_bits_remaining, self.current_length.get_bits((cursor.bit as u8)..(cursor.bit + buffer_bits_remaining) as u8) as u64);
                        cursor.bit += buffer_bits_remaining;
                        return SerializationResult::BufferFull
                    }
                }

                while (cursor.field - 1) < self.current_length*T::FLATTENED_FIELDS_NUMBER {
                    let element = (cursor.field - 1) / T::FLATTENED_FIELDS_NUMBER;
                    cursor.field -= 1 + element*T::FLATTENED_FIELDS_NUMBER;
                    match self[element].partial_serialize(cursor, buffer) {
                        SerializationResult::Finished => {
                            cursor.field += 1 + element*T::FLATTENED_FIELDS_NUMBER;
                        },
                        SerializationResult::BufferFull => {
                            cursor.field += 1 + element*T::FLATTENED_FIELDS_NUMBER;
                            return SerializationResult::BufferFull;
                        },
                    }
                }

                cursor.field = Self::FLATTENED_FIELDS_NUMBER;
                cursor.bit = 0;
                SerializationResult::Finished
            }

            fn partial_deserialize(&mut self, cursor: &mut Cursor, buffer: &mut DeserializationBuffer) -> DeserializationResult {

                // deserialize length
                if cursor.field == 0 {
                    
                    let buffer_len = buffer.bit_length();
                    if buffer_len == 0 {
                        return DeserializationResult::BufferInsufficient
                    } else if buffer_len + cursor.bit < Self::LENGTH_BITS {
                        self.deserialized_length.set_bits(cursor.bit as u8..(cursor.bit+buffer_len) as u8, buffer.pop_bits(buffer_len) as usize);
                        cursor.bit += buffer_len;
                        return DeserializationResult::BufferInsufficient
                    } else {
                        self.deserialized_length.set_bits(cursor.bit as u8..Self::LENGTH_BITS as u8, buffer.pop_bits(Self::LENGTH_BITS-cursor.bit) as usize);
                        cursor.field = 1;
                        cursor.bit = 0;
                    }
                }
                
                while (cursor.field - 1) < self.deserialized_length*T::FLATTENED_FIELDS_NUMBER {
                    let element = (cursor.field - 1) / T::FLATTENED_FIELDS_NUMBER;
                    cursor.field -= 1 + element*T::FLATTENED_FIELDS_NUMBER;
                    match self.array[element].partial_deserialize(cursor, buffer) {
                        DeserializationResult::Finished => {
                            cursor.field += 1 + element*T::FLATTENED_FIELDS_NUMBER;
                            self.current_length = element+1;
                        },
                        DeserializationResult::BufferInsufficient => {
                            cursor.field += 1 + element*T::FLATTENED_FIELDS_NUMBER;
                            return DeserializationResult::BufferInsufficient;
                        },
                    }
                }

                cursor.field = Self::FLATTENED_FIELDS_NUMBER;
                cursor.bit = 0;
                DeserializationResult::Finished
            }
            
        }

        impl<T> Index<usize> for Dynamic<[T; $size]> {
            type Output = T;
            
            fn index(&self, index: usize) -> &T {
                &self.array[0..self.current_length][index]
            }
        }
        
        impl<T> IndexMut<usize> for Dynamic<[T; $size]> {
            fn index_mut(&mut self, index: usize) -> &mut T {
                &mut self.array[0..self.current_length][index]
            }
        }

        impl<T> AsRef<[T]> for Dynamic<[T; $size]> {
            fn as_ref(&self) -> &[T] {
                &self.array[0..self.current_length]
            }
        }

        impl<T> AsMut<[T]> for Dynamic<[T; $size]> {
            fn as_mut(&mut self) -> &mut [T] {
                &mut self.array[0..self.current_length]
            }
        }

        impl<T> Default for Dynamic<[T; $size]> {
            fn default() -> Self {
                Self::new()
            }
        }

        
        // This is needed since it can't be derived for arrays larger than 32 yet
        impl<T: cmp::PartialEq> cmp::PartialEq for Dynamic<[T; $size]> {
            fn eq(&self, other: &Self) -> bool {
                if self.current_length != other.current_length {
                    return false;
                }
                
                for i in 0..self.current_length {
                    if self.array[i] != other.array[i] {
                        return false;
                    }
                }

                true
            }
        }
        
        // This is needed since it can't be derived for arrays larger than 32 yet
        impl<T: fmt::Debug> fmt::Debug for Dynamic<[T; $size]> {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "$i<T> {{ data: [")?;
                for i in 0..self.current_length {
                    write!(f, "{:?}, ", self.array[i])?;
                }
                write!(f, "]}}")
            }
        }
        
        impl<T: Clone> Clone for Dynamic<[T; $size]> {
            fn clone(&self) -> Self {
                let mut a = Self::new();
                for i in 0..self.current_length {
                    a.push(self.array[i].clone());
                }
                a
            }
        }
        
    };
}

impl<T> Drop for Dynamic<T> {
    fn drop(&mut self) {
        // Since const generics doesn't work we can't deconstruct elements inside arrays when it's beeing dropped
        // This might in extreme cases cause memory leaks and other weirdness from deconstructors not running
        // This isn't good but not UD or unsafe either.
        // Fix as soon as const generics lands
    }
}
        


impl_array!([(1, 1), (2, 2), (3, 2), (4, 3), (5, 3), (6, 3), (7, 3), (8, 4), (9, 4)]);
impl_array!([(10, 4), (11, 4), (12, 4), (13, 4), (14, 4), (15, 4), (16, 5), (17, 5), (18, 5), (19, 5)]);
impl_array!([(20, 5), (21, 5), (22, 5), (23, 5), (24, 5), (25, 5), (26, 5), (27, 5), (28, 5), (29, 5)]);
impl_array!([(30, 5), (31, 5), (32, 6), (33, 6), (34, 6), (35, 6), (36, 6), (37, 6), (38, 6), (39, 6)]);
impl_array!([(40, 6), (41, 6), (42, 6), (43, 6), (44, 6), (45, 6), (46, 6), (47, 6), (48, 6), (49, 6)]);
impl_array!([(50, 6), (51, 6), (52, 6), (53, 6), (54, 6), (55, 6), (56, 6), (57, 6), (58, 6), (59, 6)]);
impl_array!([(60, 6), (61, 6), (62, 6), (63, 6), (64, 7), (65, 7), (66, 7), (67, 7), (68, 7), (69, 7)]);
impl_array!([(70, 7), (71, 7), (72, 7), (73, 7), (74, 7), (75, 7), (76, 7), (77, 7), (78, 7), (79, 7)]);
impl_array!([(80, 7), (81, 7), (82, 7), (83, 7), (84, 7), (85, 7), (86, 7), (87, 7), (88, 7), (89, 7)]);
impl_array!([(90, 7), (91, 7), (92, 7), (93, 7), (94, 7), (95, 7), (96, 7), (97, 7), (98, 7), (99, 7)]);

impl_array!([(100, 7), (101, 7), (102, 7), (103, 7), (104, 7), (105, 7), (106, 7), (107, 7), (108, 7), (109, 7)]);
impl_array!([(110, 7), (111, 7), (112, 7), (113, 7), (114, 7), (115, 7), (116, 7), (117, 7), (118, 7), (119, 7)]);
impl_array!([(120, 7), (121, 7), (122, 7), (123, 7), (124, 7), (125, 7), (126, 7), (127, 7), (128, 8), (129, 8)]);
impl_array!([(130, 8), (131, 8), (132, 8), (133, 8), (134, 8), (135, 8), (136, 8), (137, 8), (138, 8), (139, 8)]);
impl_array!([(140, 8), (141, 8), (142, 8), (143, 8), (144, 8), (145, 8), (146, 8), (147, 8), (148, 8), (149, 8)]);
impl_array!([(150, 8), (151, 8), (152, 8), (153, 8), (154, 8), (155, 8), (156, 8), (157, 8), (158, 8), (159, 8)]);
impl_array!([(160, 8), (161, 8), (162, 8), (163, 8), (164, 8), (165, 8), (166, 8), (167, 8), (168, 8), (169, 8)]);
impl_array!([(170, 8), (171, 8), (172, 8), (173, 8), (174, 8), (175, 8), (176, 8), (177, 8), (178, 8), (179, 8)]);
impl_array!([(180, 8), (181, 8), (182, 8), (183, 8), (184, 8), (185, 8), (186, 8), (187, 8), (188, 8), (189, 8)]);
impl_array!([(190, 8), (191, 8), (192, 8), (193, 8), (194, 8), (195, 8), (196, 8), (197, 8), (198, 8), (199, 8)]);

impl_array!([(200, 8), (201, 8), (202, 8), (203, 8), (204, 8), (205, 8), (206, 8), (207, 8), (208, 8), (209, 8)]);
impl_array!([(210, 8), (211, 8), (212, 8), (213, 8), (214, 8), (215, 8), (216, 8), (217, 8), (218, 8), (219, 8)]);
impl_array!([(220, 8), (221, 8), (222, 8), (223, 8), (224, 8), (225, 8), (226, 8), (227, 8), (228, 8), (229, 8)]);
impl_array!([(230, 8), (231, 8), (232, 8), (233, 8), (234, 8), (235, 8), (236, 8), (237, 8), (238, 8), (239, 8)]);
impl_array!([(240, 8), (241, 8), (242, 8), (243, 8), (244, 8), (245, 8), (246, 8), (247, 8), (248, 8), (249, 8)]);
impl_array!([(250, 8), (251, 8), (252, 8), (253, 8), (254, 8), (255, 8), (256, 9)]);


#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void1{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void2{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void3{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void4{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void5{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void6{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void7{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void8{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void9{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void10{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void11{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void12{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void13{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void14{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void15{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void16{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void17{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void18{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void19{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void20{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void21{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void22{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void23{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void24{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void25{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void26{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void27{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void28{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void29{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void30{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void31{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void32{}

#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void33{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void34{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void35{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void36{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void37{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void38{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void39{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void40{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void41{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void42{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void43{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void44{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void45{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void46{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void47{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void48{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void49{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void50{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void51{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void52{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void53{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void54{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void55{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void56{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void57{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void58{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void59{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void60{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void61{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void62{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void63{}
#[allow(non_camel_case_types)] #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)] pub struct void64{}


macro_rules! impl_serializeable {
    {$type:ident, $bits:expr} => {
        impl SerializableStatic for $type {
            const FLATTENED_FIELDS_NUMBER: usize = 1;
        }

        impl Serializable for $type {
            fn partial_serialize(&self, cursor: &mut Cursor, buffer: &mut SerializationBuffer) -> SerializationResult {
                assert_eq!(cursor.field, 0);
                let type_bits_remaining = $bits - cursor.bit;
                let buffer_bits_remaining = buffer.bits_remaining();
                
                if type_bits_remaining == 0 {
                    cursor.bit = 0;
                    cursor.field = 1;
                    SerializationResult::Finished
                } else if buffer_bits_remaining == 0 {
                    SerializationResult::BufferFull
                } else if buffer_bits_remaining >= type_bits_remaining {
                    buffer.push_bits(type_bits_remaining, PrimitiveType::to_bits(*self) >> cursor.bit);
                    cursor.bit = 0;
                    cursor.field = 1;
                    SerializationResult::Finished
                } else {
                    buffer.push_bits(buffer_bits_remaining, PrimitiveType::to_bits(*self) >> cursor.bit);
                    cursor.bit += buffer_bits_remaining;
                    SerializationResult::BufferFull
                }
            }
            
            fn partial_deserialize(&mut self, cursor: &mut Cursor, buffer: &mut DeserializationBuffer) -> DeserializationResult {
                assert_eq!(cursor.field, 0);
                let buffer_len = buffer.bit_length();
                if buffer_len == 0 && cursor.bit == $bits {
                    cursor.bit = 0;
                    cursor.field = 1;
                    DeserializationResult::Finished
                } else if buffer_len == 0 && cursor.bit < $bits {
                    DeserializationResult::BufferInsufficient
                } else if buffer_len < $bits - cursor.bit {
                    *self = PrimitiveType::from_bits(PrimitiveType::to_bits(*self) | (buffer.pop_bits(buffer_len) << cursor.bit));
                    cursor.bit += buffer_len;
                    DeserializationResult::BufferInsufficient
                } else {
                    *self = PrimitiveType::from_bits(PrimitiveType::to_bits(*self) | (buffer.pop_bits($bits-cursor.bit) << cursor.bit));
                    cursor.bit = 0;
                    cursor.field = 1;
                    DeserializationResult::Finished
                }
            }
            
        }

    };
}

macro_rules! impl_ux{
    {[$(($type:ident, $bits:expr)),*], $underlying_type:ident} => {$(impl_ux!($type, $bits, $underlying_type);)*};
    ($type:ident, $bits:expr, $underlying_type:ident) => {
        impl PrimitiveType for $type {
            fn from_bits(v: u64) -> Self {
                $type::new(v as $underlying_type)
            }
            fn to_bits(self) -> u64 {
                u64::from(self)
            }
        }
        impl_serializeable!($type, $bits);
    };
}

macro_rules! impl_ix{
    {[$(($type:ident, $bits:expr)),*], $underlying_type:ident} => {$(impl_ix!($type, $bits, $underlying_type);)*};
    ($type:ident, $bits:expr, $underlying_type:ident) => {
        impl PrimitiveType for $type {
            fn from_bits(v: u64) -> Self {
                $type::new(v as $underlying_type)
            }
            fn to_bits(self) -> u64 {
                i64::from(self) as u64
            }
        }
        impl_serializeable!($type, $bits);
    };
}

macro_rules! impl_vx{
    {[$(($type:ident, $bits:expr)),*]} => {$(impl_vx!($type, $bits);)*};
    ($type:ident, $bits:expr) => {
        impl PrimitiveType for $type {
            fn from_bits(_v: u64) -> Self {
                $type{}
            }
            fn to_bits(self) -> u64 {
                0
            }
        }
        impl_serializeable!($type, $bits);
    };
}

impl_ux!([(u2, 2), (u3, 3), (u4, 4), (u5, 5), (u6, 6), (u7, 7)], u8);

impl_ux!([(u9, 9), (u10, 10), (u11, 11), (u12, 12), (u13, 13), (u14, 14), (u15, 15)], u16);

impl_ux!([(u17, 17), (u18, 18), (u19, 19), (u20, 20), (u21, 21), (u22, 22), (u23, 23), (u24, 24),
          (u25, 25), (u26, 26), (u27, 27), (u28, 28), (u29, 29), (u30, 30), (u31, 31)], u32);

impl_ux!([(u33, 33), (u34, 34), (u35, 35), (u36, 36), (u37, 37), (u38, 38), (u39, 39), (u40, 40),
          (u41, 41), (u42, 42), (u43, 43), (u44, 44), (u45, 45), (u46, 46), (u47, 47), (u48, 48),
          (u49, 49), (u50, 50), (u51, 51), (u52, 52), (u53, 53), (u54, 54), (u55, 55), (u56, 56),
          (u57, 57), (u58, 58), (u59, 59), (u60, 60), (u61, 61), (u62, 62), (u63, 63)], u64);



impl_ix!([(i2, 2), (i3, 3), (i4, 4), (i5, 5), (i6, 6), (i7, 7)], i8);

impl_ix!([(i9, 9), (i10, 10), (i11, 11), (i12, 12), (i13, 13), (i14, 14), (i15, 15)], i16);

impl_ix!([(i17, 17), (i18, 18), (i19, 19), (i20, 20), (i21, 21), (i22, 22), (i23, 23), (i24, 24),
          (i25, 25), (i26, 26), (i27, 27), (i28, 28), (i29, 29), (i30, 30), (i31, 31)], i32);

impl_ix!([(i33, 33), (i34, 34), (i35, 35), (i36, 36), (i37, 37), (i38, 38), (i39, 39), (i40, 40),
          (i41, 41), (i42, 42), (i43, 43), (i44, 44), (i45, 45), (i46, 46), (i47, 47), (i48, 48),
          (i49, 49), (i50, 50), (i51, 51), (i52, 52), (i53, 53), (i54, 54), (i55, 55), (i56, 56),
          (i57, 57), (i58, 58), (i59, 59), (i60, 60), (i61, 61), (i62, 62), (i63, 63)], i64);

impl_vx!([(void1, 1), (void2, 2), (void3, 3), (void4, 4), (void5, 5), (void6, 6), (void7, 7), (void8, 8)]);

impl_vx!([(void9, 9), (void10, 10), (void11, 11), (void12, 12), (void13, 13), (void14, 14), (void15, 15), (void16, 16)]);

impl_vx!([(void17, 17), (void18, 18), (void19, 19), (void20, 20), (void21, 21), (void22, 22), (void23, 23), (void24, 24),
          (void25, 25), (void26, 26), (void27, 27), (void28, 28), (void29, 29), (void30, 30), (void31, 31), (void32, 32)]);

impl_vx!([(void33, 33), (void34, 34), (void35, 35), (void36, 36), (void37, 37), (void38, 38), (void39, 39), (void40, 40),
                          (void41, 41), (void42, 42), (void43, 43), (void44, 44), (void45, 45), (void46, 46), (void47, 47), (void48, 48),
                          (void49, 49), (void50, 50), (void51, 51), (void52, 52), (void53, 53), (void54, 54), (void55, 55), (void56, 56),
                          (void57, 57), (void58, 58), (void59, 59), (void60, 60), (void61, 61), (void62, 62), (void63, 63), (void64, 64)]);


impl PrimitiveType for u8 {
    fn from_bits(v: u64) -> Self {
        v as u8
    }
    fn to_bits(self) -> u64 {
        u64::from(self)
    }
}
impl_serializeable!(u8, 8);

impl PrimitiveType for u16 {
    fn from_bits(v: u64) -> Self {
        v as u16
    }
    fn to_bits(self) -> u64 {
        u64::from(self)
    }
}
impl_serializeable!(u16, 16);
    
impl PrimitiveType for u32 {
    fn from_bits(v: u64) -> Self {
        v as u32
    }
    fn to_bits(self) -> u64 {
        u64::from(self)
    }
}
impl_serializeable!(u32, 32);

impl PrimitiveType for u64 {
    fn from_bits(v: u64) -> Self {
        v
    }
    fn to_bits(self) -> u64 {
        self
    }
}
impl_serializeable!(u64, 64);

impl PrimitiveType for i8 {
    fn from_bits(v: u64) -> Self {
        v as i8
    }
    fn to_bits(self) -> u64 {
        self as u64
    }
}
impl_serializeable!(i8, 8);

impl PrimitiveType for i16 {
    fn from_bits(v: u64) -> Self {
        v as i16
    }
    fn to_bits(self) -> u64 {
        self as u64
    }
}
impl_serializeable!(i16, 16);
    
impl PrimitiveType for i32 {
    fn from_bits(v: u64) -> Self {
        v as i32
    }
    fn to_bits(self) -> u64 {
        self as u64
    }
}
impl_serializeable!(i32, 32);

impl PrimitiveType for i64 {
    fn from_bits(v: u64) -> Self {
        v as i64
    }
    fn to_bits(self) -> u64 {
        self as u64
    }
}
impl_serializeable!(i64, 64);

impl PrimitiveType for f16 {
    fn from_bits(v: u64) -> Self {
        f16::from_bits(v as u16)
    }
    fn to_bits(self) -> u64 {
        u64::from(f16::as_bits(self))
    }
}
impl_serializeable!(f16, 16);

impl PrimitiveType for f32 {
    
    #[cfg_attr(feature="clippy", allow(transmute_int_to_float))]
    fn from_bits(v: u64) -> Self {
        let mut v32 = v as u32;
        const EXP_MASK: u32   = 0x7F80_0000;
        const FRACT_MASK: u32 = 0x007F_FFFF;
        if v32  & EXP_MASK == EXP_MASK && v32 & FRACT_MASK != 0 {
            v32 = unsafe { lib::core::mem::transmute(lib::core::f32::NAN) };
        }
        unsafe { lib::core::mem::transmute::<u32, f32>(v32) }
    }
    fn to_bits(self) -> u64 {
        (unsafe { lib::core::mem::transmute::<f32, u32>(self) }) as u64

            
    }
}
impl_serializeable!(f32, 32);

impl PrimitiveType for f64 {
    
    #[cfg_attr(feature="clippy", allow(transmute_int_to_float))]
    fn from_bits(mut v: u64) -> Self {
        const EXP_MASK: u64   = 0x7FF0_0000_0000_0000;
        const FRACT_MASK: u64 = 0x000F_FFFF_FFFF_FFFF;
        if v & EXP_MASK == EXP_MASK && v & FRACT_MASK != 0 {
            v = unsafe { lib::core::mem::transmute(lib::core::f64::NAN) };
        }
        unsafe { lib::core::mem::transmute(v) }
    }
    fn to_bits(self) -> u64 {
        unsafe { lib::core::mem::transmute(self) }
    }
}
impl_serializeable!(f64, 64);

impl PrimitiveType for bool {
    fn from_bits(v: u64) -> Self {
        v & 1 == 1
    }
    fn to_bits(self) -> u64 {
        if self {
            1
        } else {
            0
        }
    }
}
impl_serializeable!(bool, 1);




#[cfg(test)]
mod tests {

    use types::*;

    #[test]
    fn dynamic_array_with_data() {
        let a: [u8; 5] = [1, 2, 3, 4, 5];
        let d = Dynamic::<[u8; 15]>::with_data(&a);
        
        assert_eq!(d.as_ref(), &a);
    }
    
    #[test]
    fn dynamic_array_clone() {
        let a: [u8; 5] = [1, 2, 3, 4, 5];
        let d1 = Dynamic::<[u8; 15]>::with_data(&a);
        let d2 = d1.clone();
        
        assert_eq!(d1, d2);
    }
    
    #[test]
    fn dynamic_array_push() {
        let mut a = Dynamic::<[u8; 15]>::new();
        assert_eq!(a.as_ref(), &[]);

        a.push(12);
        assert_eq!(a.as_ref(), &[12]);
        
        a.push(120);
        assert_eq!(a.as_ref(), &[12, 120]);
    }
}
