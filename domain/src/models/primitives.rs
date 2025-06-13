use std::hash::Hash;
use std::marker::PhantomData;

use garde::Validate as _;
use serde::{Deserialize, Serialize, Serializer};
use uuid::Uuid;

/// ID
#[derive(Debug, Clone)]
pub struct Id<T>(pub Uuid, PhantomData<T>);

impl<T> Copy for Id<T> where T: Clone {}

impl<T> Default for Id<T> {
    fn default() -> Self {
        Self(Uuid::new_v4(), PhantomData)
    }
}

impl<T> From<Uuid> for Id<T> {
    fn from(uuid: Uuid) -> Self {
        Id(uuid, PhantomData)
    }
}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T> Eq for Id<T> {}

impl<T> PartialEq<Uuid> for Id<T> {
    fn eq(&self, other: &Uuid) -> bool {
        self.0 == *other
    }
}

impl<T> Hash for Id<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl<T> std::fmt::Display for Id<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<T> Serialize for Id<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de, T> Deserialize<'de> for Id<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let uuid = Uuid::deserialize(deserializer)?;
        Ok(Id(uuid, PhantomData))
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

        impl std::cmp::PartialEq for $name {
            fn eq(&self, other: &Self) -> bool {
                self.0 == other.0
            }
        }

        impl std::cmp::Eq for $name {}

        impl std::cmp::PartialEq<&str> for $name {
            fn eq(&self, other: &&str) -> bool {
                self.0 == *other
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

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl serde::ser::Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
            where
                S: serde::ser::Serializer,
            {
                serializer.serialize_str(&self.0)
            }
        }

        impl<'de> serde::de::Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let value: String = String::deserialize(deserializer)?;
                $name::new(value).map_err(serde::de::Error::custom)
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

        impl std::marker::Copy for $name where $name: std::clone::Clone {}

        impl std::cmp::PartialEq for $name {
            fn eq(&self, other: &Self) -> bool {
                self.0 == other.0
            }
        }

        impl std::cmp::Eq for $name {}

        impl std::cmp::PartialEq<$ty> for $name {
            fn eq(&self, other: &$ty) -> bool {
                self.0 == *other
            }
        }

        impl std::hash::Hash for $name {
            fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
                self.0.hash(state);
            }
        }

        impl std::convert::TryFrom<$ty> for $name {
            type Error = $crate::DomainError;

            fn try_from(value: $ty) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl serde::ser::Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
            where
                S: serde::ser::Serializer,
            {
                match std::any::type_name_of_val(&self.0) {
                    "i16" => serializer
                        .serialize_i16(self.0.try_into().map_err(serde::ser::Error::custom)?),
                    "i32" => serializer
                        .serialize_i32(self.0.try_into().map_err(serde::ser::Error::custom)?),
                    "i64" => serializer
                        .serialize_i64(self.0.try_into().map_err(serde::ser::Error::custom)?),
                    "u16" => serializer
                        .serialize_u16(self.0.try_into().map_err(serde::ser::Error::custom)?),
                    "u32" => serializer
                        .serialize_u32(self.0.try_into().map_err(serde::ser::Error::custom)?),
                    "u64" => serializer
                        .serialize_u64(self.0.try_into().map_err(serde::ser::Error::custom)?),
                    _ => Err(serde::ser::Error::custom("Unsupported integer type")),
                }
            }
        }

        impl<'de> serde::de::Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let value: $ty = <$ty>::deserialize(deserializer)?;
                $name::new(value).map_err(serde::de::Error::custom)
            }
        }
    };
}

/// 説明
#[derive(Debug, Clone, garde::Validate)]
pub struct Description(#[garde(length(chars, min = 1, max = 255))] pub String);
impl_string_primitive!(Description);

/// 表示順
#[derive(Debug, Clone, garde::Validate)]
pub struct DisplayOrder(#[garde(range(min=1,max=i16::MAX))] pub i16);
impl_int_primitive!(DisplayOrder, i16);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_default_ok() {
        let id = Id::<i32>::default();
        assert!(!id.0.is_nil());
    }

    #[test]
    fn id_from_uuid() {
        let value = Uuid::new_v4();
        let id = Id::<i32>::from(value);
        assert_eq!(id.0, value);
    }

    #[test]
    fn id_eq_ok() {
        let id1 = Id::<i32>::default();
        let s = id1.to_string();
        let value = Uuid::parse_str(&s).unwrap();
        assert_eq!(id1, value);
        let id2 = Id::<i32>::default();
        assert_ne!(id1, id2);
    }

    #[test]
    fn id_partial_eq_for_uuid_ok() {
        let uuid = Uuid::new_v4();
        let id = Id::<i32>::from(uuid);
        assert_eq!(id, uuid);
    }

    #[test]
    fn id_hash_ok() {
        use std::collections::HashMap;
        let key1 = Id::<i32>::default();
        let value1 = String::from("value1");
        let key2 = Id::<i32>::default();
        let value2 = String::from("value2");
        let mut map = HashMap::new();
        map.insert(key1, value1);
        map.insert(key2, value2);
        assert_eq!(map.get(&key1).unwrap(), "value1");
        assert_eq!(map.get(&key2).unwrap(), "value2");
    }

    #[test]
    fn id_display_ok() {
        let id = Id::<String>::default();
        let s = id.to_string();
        let value = Uuid::parse_str(&s).unwrap();
        assert_eq!(id.0, value);
    }

    #[test]
    fn id_deserialize_and_serialize_ok() {
        let s = "fd9e8d18-2a06-4fa0-a6cd-ec307b16caf9";
        let expected_de = Uuid::parse_str(s).unwrap();
        let expected_se = format!("\"{s}\"",);

        let deserialized = serde_json::from_str::<Id<i32>>(&expected_se).unwrap();
        assert_eq!(deserialized.0, expected_de);
        let serialized = serde_json::to_string(&deserialized).unwrap();
        assert_eq!(serialized, expected_se);
    }

    #[derive(Debug, Clone, garde::Validate)]
    pub struct StringPrimitive(#[garde(length(chars, min = 1, max = 100))] String);
    impl_string_primitive!(StringPrimitive);

    #[rstest::rstest]
    #[case(String::from("title"), true)]
    #[case(String::new(), false)]
    #[case(String::from("a"), true)]
    #[case("a".repeat(100), true)]
    #[case("a".repeat(101), false)]
    #[case("🙂".repeat(100), true)]
    #[case("🙂".repeat(100) + &String::from("a"), false)]
    fn string_primitive_new_ok(#[case] s: String, #[case] expected: bool) {
        let primitive = StringPrimitive::new(s);
        assert_eq!(primitive.is_ok(), expected);
    }

    #[test]
    fn string_primitive_eq_ok() {
        let value1 = StringPrimitive::new(String::from("value1")).unwrap();
        assert_eq!(
            value1,
            StringPrimitive::new(String::from("value1")).unwrap()
        );
        assert_ne!(
            value1,
            StringPrimitive::new(String::from("value2")).unwrap()
        );
    }

    #[test]
    fn string_primitive_partial_eq_for_str_ok() {
        let primitive = StringPrimitive::new(String::from("test")).unwrap();
        assert!(primitive == "test");
        assert!(primitive != "other");
    }

    #[test]
    fn string_primitive_try_from_ok() {
        let primitive = StringPrimitive::try_from(String::from("test"));
        assert!(primitive.is_ok());
        assert_eq!(primitive.unwrap().0, "test");
    }

    #[test]
    fn string_primitive_deref_ok() {
        let primitive = StringPrimitive::new(String::from("test")).unwrap();
        assert_eq!(&*primitive, "test");
    }

    #[test]
    fn string_primitive_display_ok() {
        let primitive = StringPrimitive::new(String::from("test")).unwrap();
        assert_eq!(primitive.to_string(), "test");
    }

    #[test]
    fn string_primitive_serialize_ok() {
        let primitive = StringPrimitive::new(String::from("test")).unwrap();
        let serialized = serde_json::to_string(&primitive).unwrap();
        assert_eq!(serialized, "\"test\"");
    }

    #[test]
    fn string_primitive_deserialize_ok() {
        let serialized = "\"test\"";
        let primitive: StringPrimitive = serde_json::from_str(serialized).unwrap();
        assert_eq!(primitive.0, "test");
    }

    #[derive(Debug, Clone, garde::Validate)]
    struct I32Primitive(#[garde(range(min = 1, max = 10))] i32);
    impl_int_primitive!(I32Primitive, i32);
}
