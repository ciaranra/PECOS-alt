/// Simple configuration for parsing
#[derive(Clone)]
pub struct ParseConfig {
    pub includes: Vec<(String, String)>,
    pub search_paths: Vec<std::path::PathBuf>,
    pub expand_gates: bool,
    pub validate_gates: bool,
}

impl Default for ParseConfig {
    fn default() -> Self {
        Self {
            includes: vec![],
            search_paths: vec![],
            expand_gates: true,
            validate_gates: true,
        }
    }
}
