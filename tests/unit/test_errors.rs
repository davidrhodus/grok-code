#[cfg(test)]
mod tests {
    use grok_code::error::GrokError;
    use std::io;
    
    #[test]
    fn test_error_display() {
        // Test IO error
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let grok_err: GrokError = io_err.into();
        assert!(grok_err.to_string().contains("IO error"));
        
        // Test custom errors
        let api_err = GrokError::Api("test api error".to_string());
        assert_eq!(api_err.to_string(), "API error: test api error");
        
        let config_err = GrokError::Config("invalid config".to_string());
        assert_eq!(config_err.to_string(), "Configuration error: invalid config");
        
        let not_found_err = GrokError::FileNotFound("/test/path".to_string());
        assert_eq!(not_found_err.to_string(), "File not found: /test/path");
    }
    
    #[test]
    fn test_error_from_conversions() {
        // Test that From conversions work
        let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "denied");
        let _: GrokError = io_err.into();
        
        let json_err = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let _: GrokError = json_err.into();
    }
} 