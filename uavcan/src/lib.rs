#![cfg_attr(not(feature="std"), no_std)]
#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]

#![cfg_attr(feature="clippy", deny(almost_swapped))]
#![cfg_attr(feature="clippy", deny(blacklisted_name))]
#![cfg_attr(feature="clippy", deny(bool_comparison))]
#![cfg_attr(feature="clippy", deny(builtin_type_shadow))]
#![cfg_attr(feature="clippy", deny(clone_on_copy))]
#![cfg_attr(feature="clippy", deny(char_lit_as_u8))]
#![cfg_attr(feature="clippy", deny(should_assert_eq))]
#![cfg_attr(feature="clippy", deny(manual_memcpy))]
#![cfg_attr(feature="clippy", deny(unreadable_literal))]
#![cfg_attr(feature="clippy", deny(if_same_then_else))]
#![cfg_attr(feature="clippy", deny(needless_bool))]
#![cfg_attr(feature="clippy", deny(assign_op_pattern))]
#![cfg_attr(feature="clippy", deny(needless_return))]
#![cfg_attr(feature="clippy", deny(doc_markdown))]

#[allow(unused_imports)]
#[macro_use]
extern crate uavcan_derive;

extern crate bit_field;
extern crate embedded_types;
extern crate ux;
extern crate half;

mod lib {
    pub mod core {
        #[cfg(feature="std")]
        pub use std::*;
        #[cfg(not(feature="std"))]
        pub use core::*;
    }
}

mod uavcan {
    #[allow(unused_imports)]
    pub use *;
}

/// This module is only exposed so `Struct` can be derived.
/// It is not intended for use outside the derive macro and
/// must not be considered as a stable part of the API.
#[doc(hidden)]
pub use uavcan_derive::*;

pub mod transfer;
pub mod types;
mod serialize;
mod crc;
mod deserializer;
mod frame_assembler;
mod serializer;
mod frame_disassembler;
pub mod node;
pub mod storage;

use bit_field::BitField;

use transfer::TransferFrameID;

pub use serialize::Cursor;
pub use serialize::SerializableStatic;

pub use node::NodeConfig;
pub use node::NodeID;
pub use node::Node;
pub use node::SimpleNode;


/// These data type is only exposed so `Struct` can be derived.
/// It is not intended for use outside the derive macro and
/// must not be considered as a stable part of the API.
#[doc(hidden)]
pub use serializer::{
    SerializationResult,
    SerializationBuffer,        
};

/// These data type is only exposed so `Struct` can be derived.
/// It is not intended for use outside the derive macro and
/// must not be considered as a stable part of the API.
#[doc(hidden)]
pub use deserializer::{
    DeserializationResult,
    DeserializationBuffer,
};


/// The trait that needs to be implemented for all types that will be sent over Uavcan
///
/// The (de)serialization is based on flattening all structures to primitive fields
/// The serializer will then iterate over all fields and bits
pub trait Serializable {
    /// Partially serializes an Uavcan Type
    ///
    /// Start serializing from the cursor position until one of the conditions occur:
    ///  - The whole type has been serialized.
    ///  - The buffer is full.
    ///
    /// The return value will tell whether serialization was completed or buffer was filled.
    ///
    /// When this method returns, the cursor will be set to where serialization stopped.
    fn partial_serialize(&self, cursor: &mut Cursor, buffer: &mut SerializationBuffer) -> SerializationResult;

    /// Partially deserializes an Uavcan Type
    ///
    /// Start deserializing from the cursor position until one of the conditions occur:
    ///  - The whole type has been deserialized.
    ///  - The buffer is full.
    ///
    /// The return value will tell whether serialization was completed or buffer was filled.
    ///
    /// When this method returns, the cursor will be set to where deserialization stopped.
    fn partial_deserialize(&mut self, cursor: &mut Cursor, buffer: &mut DeserializationBuffer) -> DeserializationResult;
}

pub trait Struct: Sized + Serializable {
    const DSDL_SIGNATURE: u64;
    const DATA_TYPE_SIGNATURE: u64;
}

pub trait Message: Struct {
    const TYPE_ID: Option<u16>;
}

pub trait Request: Struct {
    type RESPONSE: Response;
    const TYPE_ID: Option<u8>;
}

pub trait Response: Struct {
    type REQUEST: Request;
    const TYPE_ID: Option<u8>;
}

#[derive(Debug, PartialEq)]
pub(crate) struct Frame<T: Struct> {
    id: TransferFrameID,
    body: T,
}

impl<T: Struct> Frame<T> {

    
    pub fn from_message(message: T, priority: u8, source_node: NodeID) -> Self where T: Message {
        if let Some(type_id) = T::TYPE_ID {
            let mut id = 0;
            id.set_bits(0..7, u32::from(source_node));
            id.set_bit(7, false);
            id.set_bits(8..24, u32::from(type_id));
            id.set_bits(24..29, u32::from(priority));
            
            Frame::from_parts(
                TransferFrameID::new(id),
                message,
            )
        } else {
            unimplemented!("Resolvation of type id is not supported yet")
        }
    }

    /*
    pub fn from_anonymous_message(message: T, priority: u8, discriminator: u16) -> Self where T: Message {
        if let Some(type_id) = T::TYPE_ID {
            let mut id = 0;
            id.set_bits(0..7, 0);
            id.set_bit(7, false);
            id.set_bits(8..10, u32::from(type_id));
            id.set_bits(10..24, u32::from(discriminator));
            id.set_bits(24..29, u32::from(priority));
            
            Frame::from_parts(
                TransferFrameID::new(id),
                message,
            )
        } else {
            unimplemented!("Resolvation of type id is not supported yet")
        }

    }

    pub fn from_request(request: T, priority: u8, source_node: NodeID, destination_node: NodeID) -> Self where T: Request{
        if let Some(type_id) = T::TYPE_ID {
            let mut id = 0;
            id.set_bits(0..7, u32::from(source_node));
            id.set_bit(7, false);
            id.set_bits(8..15, u32::from(destination_node));
            id.set_bit(15, true);
            id.set_bits(16..24, u32::from(type_id));
            id.set_bits(24..29, u32::from(priority));
            
            Frame::from_parts(
                TransferFrameID::new(id),
                request,
            )
        } else {
            unimplemented!("Resolvation of type id is not supported yet")
        }

    }

    pub fn from_response(response: T, priority: u8, source_node: NodeID, destination_node: NodeID) -> Self where T: Response {
        if let Some(type_id) = T::TYPE_ID {
            let mut id = 0;
            id.set_bits(0..7, u32::from(source_node));
            id.set_bit(7, false);
            id.set_bits(8..15, u32::from(destination_node));
            id.set_bit(15, true);
            id.set_bits(16..24, u32::from(type_id));
            id.set_bits(24..29, u32::from(priority));

            Frame::from_parts(
                TransferFrameID::new(id),
                response,
            )
        } else {
            unimplemented!("Resolvation of type id is not supported yet")
        }

    }
    */
    
    fn from_parts(id: TransferFrameID, body: T) -> Self {
        Frame{id: id, body: body}
    }
    
    fn into_parts(self) -> (TransferFrameID, T) {
        (self.id, self.body)
    }
}






#[cfg(test)]
mod tests {

    use *;

    // Implementing some types common for several tests
    
    #[derive(Debug, PartialEq)]
    pub struct CanFrame {
        pub id: TransferFrameID,
        pub dlc: usize,
        pub data: [u8; 8],
    }

    impl transfer::TransferFrame for CanFrame {
        const MAX_DATA_LENGTH: usize = 8;
        
        fn new(id: TransferFrameID) -> CanFrame {
            CanFrame{id: id, dlc: 0, data: [0; 8]}
        }
        
        fn set_data_length(&mut self, length: usize) {
            assert!(length <= 8);
            self.dlc = length;
        }

        fn data(&self) -> &[u8] {
            &self.data[0..self.dlc]
        }

        fn data_as_mut(&mut self) -> &mut[u8] {
            &mut self.data[0..self.dlc]
        }
        
        fn id(&self) -> TransferFrameID {
            self.id 
        }
    }

    
    
}
