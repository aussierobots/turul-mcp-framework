//! Shared utilities for declarative macros

/// Capitalize the first character of a string
pub fn capitalize(s: &str) -> String {
    snake_to_pascal_case(s)
}

/// Convert snake_case to PascalCase
pub fn snake_to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars).collect(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capitalize() {
        assert_eq!(capitalize("hello"), "Hello");
        assert_eq!(capitalize("world"), "World");
        assert_eq!(capitalize(""), "");
        assert_eq!(capitalize("a"), "A");
        assert_eq!(capitalize("ALREADY"), "ALREADY");
        // Test snake_case conversion
        assert_eq!(capitalize("text_editor"), "TextEditor");
        assert_eq!(capitalize("user_details"), "UserDetails");
        assert_eq!(capitalize("resources_changed"), "ResourcesChanged");
        assert_eq!(capitalize("file_logger"), "FileLogger");
    }
}