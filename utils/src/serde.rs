use secrecy::{ExposeSecret as _, SecretString};
use serde::{Deserialize, Deserializer, Serializer};
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
