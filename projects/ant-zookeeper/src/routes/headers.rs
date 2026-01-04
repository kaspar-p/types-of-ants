use std::str::FromStr;

use ant_zoo_storage::host_architecture::HostArchitecture;
use axum_extra::headers::Header;
use http::{HeaderName, HeaderValue};

static X_ANT_PROJECT_HEADER: HeaderName = http::HeaderName::from_static("x-ant-project");
pub struct XAntProjectHeader(pub String);

impl Header for XAntProjectHeader {
    fn name() -> &'static http::HeaderName {
        &X_ANT_PROJECT_HEADER
    }

    fn decode<'i, I>(values: &mut I) -> Result<Self, axum_extra::headers::Error>
    where
        Self: Sized,
        I: Iterator<Item = &'i http::HeaderValue>,
    {
        let value = values
            .next()
            .ok_or_else(axum_extra::headers::Error::invalid)?;

        let value = value
            .to_str()
            .map_err(|_| axum_extra::headers::Error::invalid())?
            .to_string();

        Ok(Self(value))
    }

    fn encode<E: Extend<http::HeaderValue>>(&self, values: &mut E) {
        let value = HeaderValue::from_str(&self.0).expect("invalid header value stored");
        values.extend(std::iter::once(value));
    }
}

static X_ANT_VERSION_HEADER: HeaderName = http::HeaderName::from_static("x-ant-version");
pub struct XAntVersionHeader(pub String);

impl Header for XAntVersionHeader {
    fn name() -> &'static http::HeaderName {
        &X_ANT_VERSION_HEADER
    }

    fn decode<'i, I>(values: &mut I) -> Result<Self, axum_extra::headers::Error>
    where
        Self: Sized,
        I: Iterator<Item = &'i http::HeaderValue>,
    {
        let value = values
            .next()
            .ok_or_else(axum_extra::headers::Error::invalid)?;

        let value = value
            .to_str()
            .map_err(|_| axum_extra::headers::Error::invalid())?
            .to_string();

        Ok(Self(value))
    }

    fn encode<E: Extend<http::HeaderValue>>(&self, values: &mut E) {
        let value = HeaderValue::from_str(&self.0).expect("invalid header value stored");
        values.extend(std::iter::once(value));
    }
}

static X_ANT_ARCHITECTURE_HEADER: HeaderName = http::HeaderName::from_static("x-ant-architecture");
pub struct XAntArchitectureHeader(pub Option<HostArchitecture>);

impl Header for XAntArchitectureHeader {
    fn name() -> &'static http::HeaderName {
        &X_ANT_ARCHITECTURE_HEADER
    }

    fn decode<'i, I>(values: &mut I) -> Result<Self, axum_extra::headers::Error>
    where
        Self: Sized,
        I: Iterator<Item = &'i http::HeaderValue>,
    {
        let value = values.next();

        if let Some(value) = value {
            let value = value
                .to_str()
                .map_err(|_| axum_extra::headers::Error::invalid())?
                .to_string()
                .to_lowercase();

            return Ok(Self(Some(
                HostArchitecture::from_str(&value)
                    .map_err(|_| axum_extra::headers::Error::invalid())?,
            )));
        } else {
            return Ok(Self(None));
        }
    }

    fn encode<E: Extend<http::HeaderValue>>(&self, values: &mut E) {
        if let Some(value) = &self.0 {
            let value =
                HeaderValue::from_str(&value.as_str()).expect("invalid header value stored");
            values.extend(std::iter::once(value));
        }
    }
}
