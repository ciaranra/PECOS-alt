/// Helper trait for aligned pointer casts
pub trait AlignedCast {
    /// Cast a pointer to a different type while ensuring proper alignment
    ///
    /// # Safety
    /// The caller must ensure that:
    /// - The memory pointed to contains valid data for type T
    /// - The memory region is valid for the size of T
    /// - The lifetime of the pointee outlives the returned pointer
    unsafe fn cast_aligned<T>(self) -> *const T;
}

impl AlignedCast for *const u8 {
    unsafe fn cast_aligned<T>(self) -> *const T {
        // Calculate required alignment
        let align = std::mem::align_of::<T>();
        let offset = (self as usize) % align;

        if offset != 0 {
            // If not aligned, calculate padding needed
            let padding = align - offset;
            self.add(padding).cast::<T>()
        } else {
            // Already aligned
            self.cast::<T>()
        }
    }
}

impl AlignedCast for *mut u8 {
    unsafe fn cast_aligned<T>(self) -> *const T {
        self.cast_const().cast_aligned::<T>()
    }
}
