use postgres_types::{FromSql, ToSql};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

// #[derive(
//     Debug, Serialize, Clone, Copy, Deserialize, Hash, Eq, PartialEq, PartialOrd, Ord, FromSql, ToSql,
// )]
// pub struct NumId(pub u32);

#[derive(
    Debug, Serialize, Clone, Deserialize, Hash, Eq, PartialEq, PartialOrd, Ord, FromSql, ToSql,
)]
pub struct Id(pub Uuid);

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
