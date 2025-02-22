/// Trait for extracting basic metadata from a struct
pub trait StructMetadata {
    /// Get the name of the struct (typically the struct identifier)
    fn name(&self) -> &str;

    /// Get the description of the struct (typically from doc comments)
    fn description(&self) -> &str;
}
