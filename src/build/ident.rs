use crate::error::{ParseError, Result};

pub fn entity_name_to_ident(name: &str) -> Result<String> {
    let sanitized: String = name
        .chars()
        .map(|c| if c == '-' { '_' } else { c })
        .collect();
    if sanitized.is_empty() || !is_valid_ident(&sanitized) {
        return Err(ParseError::InvalidEntityName {
            name: name.to_string(),
        });
    }
    Ok(sanitized)
}

fn is_valid_ident(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c == '_' || c.is_ascii_alphabetic() => {}
        _ => return false,
    }
    chars.all(|c| c == '_' || c.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_names() {
        assert!(entity_name_to_ident("dummy_model").is_ok());
        assert!(entity_name_to_ident("dummy_model__default_transformation").is_ok());
    }

    #[test]
    fn sanitizes_hyphens() {
        assert_eq!(entity_name_to_ident("my-model").unwrap(), "my_model");
    }

    #[test]
    fn rejects_invalid_names() {
        assert!(entity_name_to_ident("123bad").is_err());
        assert!(entity_name_to_ident("").is_err());
    }
}
