use super::Processor;
use crate::message::ptr::AlignedCast;
use crate::message::{MessageBatch, MessageHeader, MessageType};

/// Composite processor that coordinates between two other processors
pub struct CompositeProcessor<P1: Processor, P2: Processor> {
    pub processor1: P1,
    pub processor2: P2,
}

#[allow(clippy::cast_sign_loss)]
#[allow(clippy::cast_ptr_alignment)]
impl<P1: Processor, P2: Processor> Processor for CompositeProcessor<P1, P2> {
    fn process(&mut self, batch: MessageBatch) -> MessageBatch {
        let mut current_batch = self.processor1.process(batch);

        loop {
            // Examine current message header
            let header = unsafe { &*current_batch.data.cast_aligned::<MessageHeader>() };
            println!(
                "Composite processor received message type: {:?}",
                header.msg_type
            );

            match header.msg_type {
                MessageType::Input => {
                    // P1 has input for P2
                    let p2_result = self.processor2.process(current_batch);
                    current_batch = self.processor1.process(p2_result);
                }
                MessageType::Halted | MessageType::Error | MessageType::Panic => {
                    // Terminal states - return directly
                    return current_batch;
                }
                _ => {
                    // Other messages (including measurements) get passed back to P1
                    current_batch = self.processor1.process(current_batch);
                }
            }
        }
    }
}
