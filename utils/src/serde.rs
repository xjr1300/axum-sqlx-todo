use std::{fmt::Display, str::FromStr};

use secrecy::{ExposeSecret as _, SecretString};
use serde::{Deserialize, Deserializer, Serializer, de::Error};
use time::{OffsetDateTime, serde::rfc3339};

pub fn serialize_option_offset_datetime<S>(
    dt: &Option<OffsetDateTime>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match dt {
        Some(dt) => rfc3339::serialize(dt, serializer),
        None => serializer.serialize_none(),
    }
}

pub fn deserialize_option_offset_datetime<'de, D>(
    deserializer: D,
) -> Result<Option<OffsetDateTime>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct Wrapper(#[serde(deserialize_with = "rfc3339::deserialize")] OffsetDateTime);

    let value: Option<Wrapper> = Option::deserialize(deserializer)?;
    Ok(value.map(|Wrapper(dt)| dt))
}

pub fn serialize_secret_string<S>(s: &SecretString, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(s.expose_secret())
}

pub fn deserialize_secret_string<'de, D>(deserializer: D) -> Result<SecretString, D::Error>
where
    D: Deserializer<'de>,
{
    let value: String = String::deserialize(deserializer)?;
    Ok(SecretString::new(value.into()))
}

pub fn deserialize_split_comma<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
    <T as FromStr>::Err: Display,
{
    let s = String::deserialize(deserializer)?;
    let value = s
        .split(',')
        .map(|x| T::from_str(x.trim()))
        .collect::<Result<Vec<T>, _>>()
        .map_err(Error::custom)?;
    Ok(value)
}

pub fn deserialize_option_split_comma<'de, D, T>(
    deserializer: D,
) -> Result<Option<Vec<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
    <T as FromStr>::Err: Display,
{
    #[derive(Deserialize)]
    struct Wrapper<T>(#[serde(deserialize_with = "deserialize_split_comma")] Vec<T>)
    where
        T: FromStr,
        <T as FromStr>::Err: Display;

    let values: Option<Wrapper<T>> = Option::deserialize(deserializer)?;
    Ok(values.map(|Wrapper(values)| values))
}
