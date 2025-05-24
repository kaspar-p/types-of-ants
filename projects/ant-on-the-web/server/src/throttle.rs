use std::net::IpAddr;

use tower_governor::{
    key_extractor::{KeyExtractor, SmartIpKeyExtractor},
    GovernorError,
};

#[derive(Clone)]
pub struct ThrottleExtractor {
    inner: SmartIpKeyExtractor,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum ThrottleKey {
    Ip(IpAddr),
    S(String),
}

impl ThrottleExtractor {
    pub fn new() -> Self {
        ThrottleExtractor {
            inner: SmartIpKeyExtractor,
        }
    }
}

impl KeyExtractor for ThrottleExtractor {
    type Key = ThrottleKey;

    fn extract<T>(
        &self,
        req: &http::Request<T>,
    ) -> Result<Self::Key, tower_governor::GovernorError> {
        let k = match self.inner.extract(req) {
            Ok(r) => ThrottleKey::Ip(r),
            Err(GovernorError::UnableToExtractKey) => ThrottleKey::S("ant-on-the-web".to_owned()),
            Err(e) => return Err(e),
        };

        Ok(k)
    }

    fn key_name(&self, key: &Self::Key) -> Option<String> {
        match key {
            ThrottleKey::Ip(k) => self.inner.key_name(k),
            ThrottleKey::S(s) => Some(s.clone()),
        }
    }

    fn name(&self) -> &'static str {
        "ant-on-the-web"
    }
}
