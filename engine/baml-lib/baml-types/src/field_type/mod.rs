use crate::BamlMediaType;

mod builder;

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
pub enum TypeValue {
    String,
    Int,
    Float,
    Bool,
    // Char,
    Null,
    Media(BamlMediaType),
}
impl TypeValue {
    pub fn from_str(s: &str) -> Option<TypeValue> {
        match s {
            "string" => Some(TypeValue::String),
            "int" => Some(TypeValue::Int),
            "float" => Some(TypeValue::Float),
            "bool" => Some(TypeValue::Bool),
            "null" => Some(TypeValue::Null),
            "image" => Some(TypeValue::Media(BamlMediaType::Image)),
            "audio" => Some(TypeValue::Media(BamlMediaType::Audio)),
            _ => None,
        }
    }
}
impl std::fmt::Display for TypeValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeValue::String => write!(f, "string"),
            TypeValue::Int => write!(f, "int"),
            TypeValue::Float => write!(f, "float"),
            TypeValue::Bool => write!(f, "bool"),
            TypeValue::Null => write!(f, "null"),
            TypeValue::Media(BamlMediaType::Image) => write!(f, "image"),
            TypeValue::Media(BamlMediaType::Audio) => write!(f, "audio"),
        }
    }
}

/// FieldType represents the type of either a class field or a function arg.
#[derive(serde::Serialize, Debug, Clone)]
pub enum FieldType {
    Primitive(TypeValue),
    Enum(String),
    Class(String),
    List(Box<FieldType>),
    Map(Box<FieldType>, Box<FieldType>),
    Union(Vec<FieldType>),
    Tuple(Vec<FieldType>),
    Optional(Box<FieldType>),
}

// Impl display for FieldType
impl std::fmt::Display for FieldType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FieldType::Enum(name) | FieldType::Class(name) => {
                write!(f, "{}", name)
            }
            FieldType::Primitive(t) => write!(f, "{}", t),
            FieldType::Union(choices) => {
                write!(
                    f,
                    "({})",
                    choices
                        .iter()
                        .map(|t| t.to_string())
                        .collect::<Vec<_>>()
                        .join(" | ")
                )
            }
            FieldType::Tuple(choices) => {
                write!(
                    f,
                    "({})",
                    choices
                        .iter()
                        .map(|t| t.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            FieldType::Map(k, v) => write!(f, "map<{}, {}>", k.to_string(), v.to_string()),
            FieldType::List(t) => write!(f, "{}[]", t.to_string()),
            FieldType::Optional(t) => write!(f, "{}?", t.to_string()),
        }
    }
}

impl FieldType {
    pub fn is_primitive(&self) -> bool {
        match self {
            FieldType::Primitive(_) => true,
            FieldType::Optional(t) => t.is_primitive(),
            FieldType::List(t) => t.is_primitive(),
            _ => false,
        }
    }

    pub fn is_optional(&self) -> bool {
        match self {
            FieldType::Optional(_) => true,
            FieldType::Primitive(TypeValue::Null) => true,

            FieldType::Union(types) => types.iter().any(FieldType::is_optional),
            _ => false,
        }
    }

    pub fn is_null(&self) -> bool {
        match self {
            FieldType::Primitive(TypeValue::Null) => true,
            FieldType::Optional(t) => t.is_null(),
            _ => false,
        }
    }
}
