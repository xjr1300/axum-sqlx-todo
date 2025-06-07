use std::marker::PhantomData;

use garde::Validate as _;
use uuid::Uuid;

/// ID
#[derive(Debug, Clone)]
pub struct Id<T>(pub Uuid, PhantomData<T>);

impl<T> Default for Id<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> std::fmt::Display for Id<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<T> Id<T> {
    fn new() -> Self {
        Id(Uuid::new_v4(), PhantomData)
    }
}

impl<T> From<Uuid> for Id<T> {
    fn from(uuid: Uuid) -> Self {
        Id(uuid, PhantomData)
    }
}

#[macro_export]
macro_rules! impl_string_primitive {
    ($name:ident) => {
        impl $name {
            pub fn new(value: std::string::String) -> $crate::DomainResult<Self> {
                let value = if $crate::starts_or_ends_with_whitespace(&value) {
                    value.trim().to_string()
                } else {
                    value
                };
                let value = Self(value);
                match value.validate() {
                    Ok(_) => Ok(value),
                    Err(e) => Err($crate::DomainError {
                        kind: $crate::DomainErrorKind::Validation,
                        messages: vec![e.to_string().into()],
                        source: e.into(),
                    }),
                }
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl std::convert::TryFrom<String> for $name {
            type Error = $crate::DomainError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl std::ops::Deref for $name {
            type Target = str;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
    };
}

#[macro_export]
macro_rules! impl_int_primitive {
    ($name:ident, $ty:ty) => {
        impl $name {
            pub fn new(value: $ty) -> $crate::DomainResult<Self> {
                let value = Self(value);
                match value.validate() {
                    Ok(_) => Ok(value),
                    Err(e) => Err($crate::DomainError {
                        kind: $crate::DomainErrorKind::Validation,
                        messages: vec![e.to_string().into()],
                        source: e.into(),
                    }),
                }
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl std::convert::TryFrom<$ty> for $name {
            type Error = $crate::DomainError;

            fn try_from(value: $ty) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }
    };
}

/// 説明
#[derive(Debug, Clone, garde::Validate)]
pub struct Description(#[garde(length(chars, min = 1, max = 255))] pub String);
impl_string_primitive!(Description);

/// 表示順
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, garde::Validate)]
pub struct DisplayOrder(#[garde(range(min=1,max=i16::MAX))] pub i16);
impl_int_primitive!(DisplayOrder, i16);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_default() {
        let id = Id::<String>::default();
        assert!(!id.0.is_nil());
    }

    #[test]
    fn id_from_uuid() {
        let value = Uuid::new_v4();
        let id = Id::<String>::from(value);
        assert_eq!(id.0, value);
    }

    #[derive(Debug, Clone, garde::Validate)]
    pub struct StringPrimitive(#[garde(length(chars, min = 1, max = 100))] pub String);
    impl_string_primitive!(StringPrimitive);

    #[rstest::rstest]
    #[case(String::from("title"), true)]
    #[case(String::new(), false)]
    #[case(String::from("a"), true)]
    #[case("a".repeat(100), true)]
    #[case("a".repeat(101), false)]
    #[case("🙂".repeat(100), true)]
    #[case("🙂".repeat(100) + &String::from("a"), false)]
    fn impl_string_primitive(#[case] s: String, #[case] expected: bool) {
        let primitive = StringPrimitive::new(s);
        assert_eq!(primitive.is_ok(), expected);
    }
}
