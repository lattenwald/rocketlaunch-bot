use std::fmt::{self, Display};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, TimestampSeconds};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Provider {
    pub id: u64,
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Vehicle {
    pub id: u64,
    pub name: String,
    pub company_id: u64,
    pub slug: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Location {
    pub id: u64,
    pub name: String,
    pub state: Option<String>,
    pub state_name: Option<String>,
    pub country: String,
    pub slug: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Pad {
    pub id: u64,
    pub name: String,
    pub location: Location,
}

impl Display for Pad {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}, {}", self.location.name, self.name)?;
        if let Some(state_name) = &self.location.state_name {
            write!(f, ", {}", state_name)?;
        }
        write!(f, ", {}", self.location.country)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Mission {
    pub id: u64,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EstimatedDate {
    pub month: Option<i32>,
    pub day: Option<i32>,
    pub year: Option<i32>,
    pub quarter: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Tag {
    pub id: u64,
    pub text: String,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Launch {
    pub id: u64,
    #[serde_as(as = "TimestampSeconds<String>")]
    pub sort_date: DateTime<Utc>,
    pub name: String,
    pub provider: Provider,
    pub vehicle: Vehicle,
    pub pad: Pad,
    pub missions: Vec<Mission>,
    pub mission_description: Option<String>,
    pub launch_description: String,
    #[serde(with = "t0_nullable")]
    pub t0: Option<DateTime<Utc>>,
    pub est_date: EstimatedDate,
    pub date_str: String,
    pub tags: Vec<Tag>,
    pub slug: String,
    pub quicktext: String,
    pub suborbital: bool,
    #[serde(with = "t0_nullable")]
    pub win_open: Option<DateTime<Utc>>,
    #[serde(with = "t0_nullable")]
    pub win_close: Option<DateTime<Utc>>,
    pub modified: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Launches {
    #[serde(rename = "result")]
    pub launches: Vec<Launch>,
}

mod t0_nullable {
    use std::fmt;

    use chrono::{DateTime, NaiveDateTime, Utc};
    use serde::{
        de::{self, Visitor},
        Deserializer, Serializer,
    };

    const FORMAT: &str = "%Y-%m-%dT%H:%MZ";

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct DatetimeOrNull;

        impl<'de> Visitor<'de> for DatetimeOrNull {
            type Value = Option<DateTime<Utc>>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("datetime or null")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let dt = NaiveDateTime::parse_from_str(value, FORMAT)
                    .map_err(serde::de::Error::custom)?;
                Ok(Some(DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc)))
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(None)
            }
        }

        deserializer.deserialize_any(DatetimeOrNull)
    }

    pub fn serialize<S>(date: &Option<DateTime<Utc>>, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if let Some(ref d) = *date {
            return s.serialize_str(&d.format(FORMAT).to_string());
        }
        s.serialize_none()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RLError {
    #[error("reqwest -> {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("json -> {0}")]
    Json(#[from] serde_json::Error),

    #[error("sled -> {0}")]
    Sled(#[from] sled::Error),

    #[error("teloxide -> {0}")]
    Teloxide(#[from] teloxide::RequestError),
}
