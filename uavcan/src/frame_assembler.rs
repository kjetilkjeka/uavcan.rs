use crc::TransferCRC;

use transfer::{
    TransferFrame,
    TransferFrameID,
    TransferID,
};

use {
    Frame,
    Struct,
};

use deserializer::{
    Deserializer,
};

#[derive(Debug, PartialEq, Eq)]
pub enum AssemblerResult {
    Ok,
    Finished,
}

#[derive(Debug, PartialEq, Eq)]
pub enum AssemblerError {
    FirstFrameNotStartFrame,
    FrameAfterEndFrame,
    IDError,
    ToggleError,
}

#[derive(Debug, PartialEq, Eq)]
pub enum BuildError {
    CRCError,
    NotFinishedParsing,
}

pub(crate) struct FrameAssembler<S: Struct> {
    deserializer: Deserializer<S>,
    started: bool,
    finished: bool,
    id: TransferFrameID,
    crc_received: Option<TransferCRC>,
    crc_calculated: TransferCRC,
    toggle: bool,
    transfer_id: TransferID,    
}

impl<S: Struct> FrameAssembler<S> {
    pub fn new() -> Self {
        Self{
            deserializer: Deserializer::new(),
            started: false,
            finished: false,
            id: TransferFrameID::new(0x00),
            crc_received: None,
            crc_calculated: TransferCRC::from_signature(S::DATA_TYPE_SIGNATURE),
            toggle: false,
            transfer_id: TransferID::new(0x00),
        }
    }
    
    pub fn add_transfer_frame<T: TransferFrame>(&mut self, mut frame: T) -> Result<AssemblerResult, AssemblerError> {
        let end_frame = frame.is_end_frame();
        
        if self.finished {
            return Err(AssemblerError::FrameAfterEndFrame);
        }
        
        if !self.started {
            if !frame.is_start_frame() {
                return Err(AssemblerError::FirstFrameNotStartFrame);
            }
            
            if frame.tail_byte().toggle() {
                return Err(AssemblerError::ToggleError);
            }
            
            if !end_frame {
                self.crc_received = Some(TransferCRC::from((frame.data()[0] as u16) | (frame.data()[1] as u16) << 8));
            }
            
            self.toggle = false;
            self.transfer_id = frame.tail_byte().transfer_id();
            self.id = frame.id();
            self.started = true;
        }

        if self.id != frame.id() {
            return Err(AssemblerError::IDError);
        }

        let data_len = frame.data().len();
        let payload = if frame.is_start_frame() && !frame.is_end_frame() {
            &mut frame.data_as_mut()[2..data_len-1]
        } else {
            &mut frame.data_as_mut()[0..data_len-1]
        };

        self.crc_calculated.add(payload);
        self.deserializer.deserialize(payload);            

        if end_frame {
            self.finished = true;
            Ok(AssemblerResult::Finished)
        } else {
            Ok(AssemblerResult::Ok)
        }
    }

    pub fn build(self) -> Result<Frame<S>, BuildError> {
        if self.crc_calculated != self.crc_received.unwrap_or(self.crc_calculated) {
            Result::Err(BuildError::CRCError)
        } else if let Ok(body) = self.deserializer.into_structure() {
            Ok(Frame::from_parts(self.id, body))
        } else {
            Err(BuildError::NotFinishedParsing)
        }
    }                
}


#[cfg(test)]
mod tests {

    use tests::{
        CanFrame,
    };
    
    use *;    
    use types::*;

    use transfer::{
        TransferID,
        TailByte,
    };

    use frame_assembler::*;
    
    #[test]
    fn parse_from_can_frames_simple() {

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
        
        let can_frame = CanFrame{id: TransferFrameID::new(0), dlc: 8, data: [1, 0, 0, 0, 0b10011100, 5, 0, TailByte::new(true, true, false, TransferID::new(0)).into()]};
        
        let mut message_builder = FrameAssembler::new();
        message_builder.add_transfer_frame(can_frame).unwrap();
        let parsed_message: Frame<NodeStatus> = message_builder.build().unwrap();
        
        assert_eq!(parsed_message.body.uptime_sec, 1);
        assert_eq!(parsed_message.body.health, u2::new(2));
        assert_eq!(parsed_message.body.mode, u3::new(3));
        assert_eq!(parsed_message.body.sub_mode, u3::new(4));
        assert_eq!(parsed_message.body.vendor_specific_status_code, 5);
        assert_eq!(parsed_message.id, TransferFrameID::new(0));
                                              
    }
    
    #[test]
    fn deserialize_multi_frame() {
        
        #[derive(Debug, PartialEq, Clone, UavcanStruct)]
        #[DSDLSignature = "0x711bf141af572346"]
        #[DataTypeSignature = "0x711bf141af572346"]
        struct LogLevel {
            value: u3,
        }

        #[derive(Debug, PartialEq, Clone, UavcanStruct)]
        #[DataTypeSignature = "0xd654a48e0c049d75"]
        #[DSDLSignature = "0xe9862b78d38762ba"]
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

        let crc = 0x69C7;
        let mut message_builder = FrameAssembler::new();
        
        message_builder.add_transfer_frame(CanFrame{
            id: TransferFrameID::new(4194080),
            dlc: 8,
            data: [crc.get_bits(0..8) as u8, crc.get_bits(8..16) as u8, 0u8.set_bits(5..8, 0).set_bits(0..5, 11).get_bits(0..8), b't', b'e', b's', b't', TailByte::new(true, false, false, TransferID::new(0)).into()],
        }).unwrap();
        
        message_builder.add_transfer_frame(CanFrame{
            id: TransferFrameID::new(4194080),
            dlc: 8,
            data: [b' ', b's', b'o', b'u', b'r', b'c', b'e', TailByte::new(false, false, false, TransferID::new(0)).into()],
        }).unwrap();
        
        message_builder.add_transfer_frame(CanFrame{
            id: TransferFrameID::new(4194080),
            dlc: 8,
            data: [9, b't', b'e', b's', b't', b' ', b't', TailByte::new(false, false, false, TransferID::new(0)).into()],
        }).unwrap();
        
        message_builder.add_transfer_frame(CanFrame{
            id: TransferFrameID::new(4194080),
            dlc: 4,
            data: [b'e', b'x', b't', TailByte::new(false, true, true, TransferID::new(0)).into(), 0, 0, 0, 0],
        }).unwrap();

        assert_eq!(uavcan_frame.body.source.length(), 11);
        assert_eq!(Ok(uavcan_frame), message_builder.build());
        
    }
   


}

