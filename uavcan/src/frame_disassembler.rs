use bit_field::BitField;

use {
    Struct,
    Frame,
};

use transfer::{
    TransferFrame,
    TransferFrameID,
    TailByte,
    TransferID,
};

use serializer::*;



pub(crate) struct FrameDisassembler<S: Struct> {
    serializer: Serializer<S>,
    started: bool,
    finished: bool,
    id: TransferFrameID,
    toggle: bool,
    transfer_id: TransferID,
}

impl<S: Struct> FrameDisassembler<S> {
    pub fn from_uavcan_frame(frame: Frame<S>, transfer_id: TransferID) -> Self {
        let (id, body) = frame.into_parts();
        Self{
            serializer: Serializer::from_structure(body),
            started: false,
            finished: false,
            id: id,
            toggle: false,
            transfer_id: transfer_id,
        }
    }

    pub fn next_transfer_frame<T: TransferFrame>(&mut self) -> Option<T> {
        let max_data_length = T::MAX_DATA_LENGTH;
        let mut transport_frame = T::new(self.id);
        transport_frame.set_data_length(max_data_length);
        
        let first_of_multi_frame = if !self.started {
            let mut buffer = SerializationBuffer::with_empty_buffer(&mut transport_frame.data_as_mut()[0..max_data_length-1]);
            if let SerializationResult::Finished = self.serializer.peek_serialize(&mut buffer) {
                false
            } else {
                true
            }                
        } else {
            false
        };

        if self.finished {
            return None;
        } else if first_of_multi_frame {
            let crc = self.serializer.crc(S::DATA_TYPE_SIGNATURE);
            transport_frame.data_as_mut()[0] = crc.get_bits(0..8) as u8;
            transport_frame.data_as_mut()[1] = crc.get_bits(8..16) as u8;
            {
                let mut buffer = SerializationBuffer::with_empty_buffer(&mut transport_frame.data_as_mut()[2..max_data_length-1]);
                self.serializer.serialize(&mut buffer);
            }
            transport_frame.data_as_mut()[max_data_length-1] = TailByte::new(!self.started, false, self.toggle, self.transfer_id).into();
        } else {
            let (frame_length, end_of_transfer) = {
                let mut buffer = SerializationBuffer::with_empty_buffer(&mut transport_frame.data_as_mut()[0..max_data_length-1]);
                if SerializationResult::Finished == self.serializer.serialize(&mut buffer){
                    self.finished = true;
                    ((buffer.bit_length()+7)/8 + 1, true)
                } else {
                    (max_data_length, false)
                }
            };
            transport_frame.set_data_length(frame_length);
            transport_frame.data_as_mut()[frame_length-1] = TailByte::new(!self.started, end_of_transfer, self.toggle, self.transfer_id).into();
        }
        
        self.started = true;
        self.toggle = !self.toggle;
        
        Some(transport_frame)
    }
}




#[cfg(test)]
mod tests {

    use bit_field::BitField;
    
    use tests::{
        CanFrame,
    };
    
    use *;
    use types::*;
    use frame_disassembler::*;

    
    #[test]
    fn serialize_node_status_frame() {

        #[derive(Debug, PartialEq, Clone, UavcanStruct, Default)]
        struct NodeStatus {
            uptime_sec: u32,
            health: u2,
            mode: u3,
            sub_mode: u3,
            vendor_specific_status_code: u16,
        }

        impl Message for NodeStatus {
            const TYPE_ID: Option<u16> = Some(341);
        }   
        let can_frame = CanFrame{id: TransferFrameID::from(TransferFrameID::new(87328)), dlc: 8, data: [1, 0, 0, 0, 0b10011100, 5, 0, TailByte::new(true, true, false, TransferID::new(0)).into()]};

        let uavcan_frame = Frame::from_message(NodeStatus{
            uptime_sec: 1,
            health: u2::new(2),
            mode: u3::new(3),
            sub_mode: u3::new(4),
            vendor_specific_status_code: 5,
        }, 0, NodeID::new(32));

        let mut frame_generator = FrameDisassembler::from_uavcan_frame(uavcan_frame, TransferID::new(0));

        assert_eq!(frame_generator.next_transfer_frame(), Some(can_frame));
        assert_eq!(frame_generator.next_transfer_frame::<CanFrame>(), None);
        
    }
    
    #[test]
    fn serialize_multi_frame() {

        #[derive(Debug, PartialEq, Clone, UavcanStruct)]
        struct LogLevel {
            value: u3,
        }
        
        #[derive(Debug, PartialEq, Clone, UavcanStruct)]
        #[DataTypeSignature = "0xd654a48e0c049d75"]
        struct LogMessage {
            level: LogLevel,
            source: Dynamic<[u8; 31]>,
            text: Dynamic<[u8; 255]>,
        }

        impl Message for LogMessage {
            const TYPE_ID: Option<u16> = Some(16383);
        }
        
        let uavcan_frame = Frame::from_message(LogMessage{
            level: LogLevel{value: u3::new(0)},
            source: Dynamic::<[u8; 31]>::with_data("test source".as_bytes()),
            text: Dynamic::<[u8; 255]>::with_data("test text".as_bytes()),
        }, 0, NodeID::new(32));

        let mut frame_generator = FrameDisassembler::from_uavcan_frame(uavcan_frame, TransferID::new(0));

        let crc = frame_generator.serializer.crc(0xd654a48e0c049d75);

        
        assert_eq!(
            frame_generator.next_transfer_frame(),
            Some(CanFrame{
                id: TransferFrameID::new(4194080),
                dlc: 8,
                data: [crc.get_bits(0..8) as u8, crc.get_bits(8..16) as u8, 0u8.set_bits(0..5, 11).set_bits(5..8, 0).get_bits(0..8), b't', b'e', b's', b't', TailByte::new(true, false, false, TransferID::new(0)).into()],
            })
        );
        
        assert_eq!(
            frame_generator.next_transfer_frame(),
            Some(CanFrame{
                id: TransferFrameID::new(4194080),
                dlc: 8,
                data: [b' ', b's', b'o', b'u', b'r', b'c', b'e', TailByte::new(false, false, true, TransferID::new(0)).into()],
            })
        );
        
        assert_eq!(
            frame_generator.next_transfer_frame(),
            Some(CanFrame{
                id: TransferFrameID::new(4194080),
                dlc: 8,
                data: [9, b't', b'e', b's', b't', b' ', b't', TailByte::new(false, false, false, TransferID::new(0)).into()],
            })
        );
        
        assert_eq!(
            frame_generator.next_transfer_frame(),
            Some(CanFrame{
                id: TransferFrameID::new(4194080),
                dlc: 4,
                data: [b'e', b'x', b't', TailByte::new(false, true, true, TransferID::new(0)).into(), 0, 0, 0, 0],
            })
        );

        assert_eq!(frame_generator.next_transfer_frame::<CanFrame>(), None);
       
    }

}
