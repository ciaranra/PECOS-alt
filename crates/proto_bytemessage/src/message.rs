use bitflags::bitflags;
use bytemuck::{Pod, Zeroable};
use std::convert::TryFrom;

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct MessageBatchHeader {
    pub magic: u32,      // Magic Number (4 bytes)
    pub total_size: u32, // Total buffer size (4 bytes)
    pub msg_count: u16,  // Number of messages (2 bytes)
    pub version: u8,     // Format version (1 byte)
    pub flags: u8,       // Bitfield for flagging (1 byte)
}

impl MessageBatchHeader {
    #[must_use]
    pub fn new(msg_count: u16, total_size: u32) -> Self {
        Self {
            magic: 0x50_45_43_53, // "PECS" in ASCII
            total_size,
            msg_count,
            version: 1,
            flags: 0,
        }
    }
}

#[allow(dead_code)]
#[repr(u8)]
#[derive(Debug, Copy, Clone)]
pub enum MessageType {
    Example = 1,
    Other = 2,
}

impl TryFrom<u8> for MessageType {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, <Self as TryFrom<u8>>::Error> {
        match value {
            1 => Ok(Self::Example),
            _ => Err(()), // Explicitly handle unknown values
        }
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct MessageFlags: u8 {
        const NONE         = 0b0000_0000; // No special behavior
        const RESERVED_1   = 0b0000_0001; // Reserved for future use
        const RESERVED_2   = 0b0000_0010; // Reserved for future use
        const RESERVED_3   = 0b0000_0100; // Reserved for future use
        const RESERVED_4   = 0b0000_1000; // Reserved for future use
        const RESERVED_5   = 0b0001_0000; // Reserved for future use
        const RESERVED_6   = 0b0010_0000; // Reserved for future use
        const RESERVED_7   = 0b0100_0000; // Reserved for future use
        const RESERVED_8   = 0b1000_0000; // Reserved for future use
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct MessageHeader {
    pub msg_type: u8,
    pub flags: u8,
    pub msg_size: u16,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct MessageBatch {
    pub data: Vec<u8>,
}

impl MessageBatch {
    #[allow(dead_code)]
    #[must_use]
    pub fn get_header(&self) -> &MessageBatchHeader {
        let header_bytes = &self.data[0..size_of::<MessageBatchHeader>()];
        bytemuck::from_bytes(header_bytes)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&MessageHeader, &[u8])> {
        let mut offset = size_of::<MessageBatchHeader>(); // Skip batch header
        std::iter::from_fn(move || {
            if offset >= self.data.len() {
                return None;
            }

            // Read header
            let header_bytes = &self.data[offset..offset + size_of::<MessageHeader>()];
            let header: &MessageHeader = bytemuck::from_bytes(header_bytes);
            offset += size_of::<MessageHeader>();

            // Read payload
            let payload_size = header.msg_size as usize;
            let payload = &self.data[offset..offset + payload_size];
            offset += payload_size;

            // Ensure 4-byte alignment (padding)
            offset = (offset + 3) & !3;

            Some((header, payload))
        })
    }
}

#[allow(dead_code)]
#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct ExampleData {
    pub id: u32,
    pub value: u32,
}

#[derive(Debug)]
pub struct BatchBuilder {
    buffer: Vec<u8>,
    msg_count: u16,
}

impl Default for BatchBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl BatchBuilder {
    #[must_use]
    pub fn new() -> BatchBuilder {
        Self {
            buffer: Vec::new(),
            msg_count: 0,
        }
    }

    /// Helper to calculate necessary padding (4-byte alignment)
    fn padding(size: usize) -> usize {
        (4 - (size % 4)) % 4
    }

    /// Adds a new message to the batch buffer.
    ///
    /// This function appends the message header and its associated data to the internal buffer,
    /// ensuring 4-byte alignment for efficient memory access. It automatically calculates
    /// padding for both the message header and the payload.
    ///
    /// # Parameters
    /// * `msg_type` - The type of the message, represented as a `MessageType`.
    /// * `data` - A byte slice (`&[u8]`) containing the message payload.
    ///
    /// # Returns
    /// A mutable reference to `self`, allowing method chaining.
    ///
    /// # Panics
    /// This function panics if the length of `data` exceeds the maximum value that can
    /// be stored in a `u16` (65,535 bytes), as the message size is limited by the field
    /// type of `MessageHeader`.
    pub fn add(&mut self, msg_type: MessageType, data: &[u8]) -> &mut Self {
        let data_len = data.len();

        // Ensure we are using 4 byte alignment for fast memory reads
        let offset = self.buffer.len() % 4;
        if offset != 0 {
            let padding_size = Self::padding(self.buffer.len());
            self.buffer.extend(std::iter::repeat_n(0, padding_size));
        }

        let header = MessageHeader {
            msg_type: msg_type as u8,
            flags: MessageFlags::NONE.bits(),
            msg_size: u16::try_from(data_len).expect("Message size too large"),
        };
        self.buffer.extend_from_slice(bytemuck::bytes_of(&header));

        // Append message data
        self.buffer.extend_from_slice(data);

        // Ensure next message also starts at a 4-byte boundary
        self.buffer
            .extend(std::iter::repeat_n(0, Self::padding(data_len)));

        self.msg_count += 1;

        self
    }

    /// Convenience function for adding `ExampleData` message
    #[allow(dead_code)]
    pub fn add_example(&mut self, id: u32, value: u32) -> &mut Self {
        let data = ExampleData { id, value };
        let raw_data = bytemuck::bytes_of(&data); // Convert struct to Vec<u8>
        self.add(MessageType::Example, raw_data) // Reuse add()
    }

    /// Constructs and returns the final `MessageBatch` with a prepended header.
    ///
    /// This method finalizes the batch by adding a `MessageBatchHeader` at the start
    /// of the buffer and prepares the builder for reuse.
    ///
    /// # Returns
    /// A `MessageBatch` containing the serialized header and all appended messages.
    ///
    /// # Panics
    /// Panics if the total size of the batch exceeds `u32::MAX`.
    pub fn build(&mut self) -> MessageBatch {
        let total_size = u32::try_from(self.buffer.len() + size_of::<MessageBatchHeader>())
            .expect("Message size too large");

        let header = MessageBatchHeader::new(self.msg_count, total_size);

        // Resetting state so builder can be used again
        self.msg_count = 0;

        let mut final_buffer = std::mem::take(&mut self.buffer); // Efficiently take ownership

        // Ensure the final buffer has enough space for header
        final_buffer.reserve_exact(size_of::<MessageBatchHeader>());

        // Insert header at the start (avoid multiple `extend_from_slice` calls)
        final_buffer.splice(0..0, bytemuck::bytes_of(&header).iter().copied());

        MessageBatch { data: final_buffer }
    }
}
