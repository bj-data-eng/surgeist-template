#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameError {
    InvalidVariableName { name: String },
    InvalidAttributeName { name: String },
    InvalidNativeElementName { name: String },
    InvalidComponentName { name: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VariableName(String);

impl VariableName {
    pub fn try_new(name: impl Into<String>) -> Result<Self, NameError> {
        let name = name.into();
        if is_identifier(&name) {
            Ok(Self(name))
        } else {
            Err(NameError::InvalidVariableName { name })
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AttributeName(String);

impl AttributeName {
    pub fn try_new(name: impl Into<String>) -> Result<Self, NameError> {
        let name = name.into();
        if is_dash_name(&name) {
            Ok(Self(name))
        } else {
            Err(NameError::InvalidAttributeName { name })
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NativeElementName(String);

impl NativeElementName {
    pub fn try_new(name: impl Into<String>) -> Result<Self, NameError> {
        let name = name.into();
        if is_native_element_name(&name) {
            Ok(Self(name))
        } else {
            Err(NameError::InvalidNativeElementName { name })
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ComponentName(String);

impl ComponentName {
    pub fn try_new(name: impl Into<String>) -> Result<Self, NameError> {
        let name = name.into();
        if is_component_name(&name) {
            Ok(Self(name))
        } else {
            Err(NameError::InvalidComponentName { name })
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    is_identifier_start(first) && chars.all(is_identifier_continue)
}

fn is_dash_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    is_identifier_start(first) && chars.all(is_dash_name_continue)
}

fn is_native_element_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    first.is_ascii_lowercase() && chars.all(is_native_element_name_continue)
}

fn is_component_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    first.is_ascii_uppercase() && chars.all(is_identifier_continue)
}

fn is_identifier_start(value: char) -> bool {
    value == '_' || value.is_ascii_alphabetic()
}

fn is_identifier_continue(value: char) -> bool {
    value == '_' || value.is_ascii_alphanumeric()
}

fn is_dash_name_continue(value: char) -> bool {
    value == '-' || is_identifier_continue(value)
}

fn is_native_element_name_continue(value: char) -> bool {
    value == '-' || value == '_' || value.is_ascii_lowercase() || value.is_ascii_digit()
}

#[cfg(test)]
mod tests {
    use super::{AttributeName, ComponentName, NativeElementName, VariableName};

    #[test]
    fn variable_names_require_identifier_shape() {
        let name = VariableName::try_new("_item2").expect("valid variable name");

        assert_eq!(name.as_str(), "_item2");
        assert!(VariableName::try_new("2item").is_err());
    }

    #[test]
    fn attribute_names_allow_kebab_after_letter_or_underscore_start() {
        let name = AttributeName::try_new("data-id").expect("valid attribute name");

        assert_eq!(name.as_str(), "data-id");
        assert!(AttributeName::try_new("-data").is_err());
    }

    #[test]
    fn native_element_names_are_lowercase_kebab_style() {
        let name = NativeElementName::try_new("custom-element").expect("valid native name");

        assert_eq!(name.as_str(), "custom-element");
        assert!(NativeElementName::try_new("Div").is_err());
        assert!(NativeElementName::try_new("dIv").is_err());
    }

    #[test]
    fn component_names_require_uppercase_identifier_shape() {
        let name = ComponentName::try_new("PanelHeader").expect("valid component name");

        assert_eq!(name.as_str(), "PanelHeader");
        assert!(ComponentName::try_new("panelHeader").is_err());
        assert!(ComponentName::try_new("Panel-Header").is_err());
    }
}
