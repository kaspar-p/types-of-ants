pub use super::lib::Id as HostId;
use crate::dao::{dao_trait::DaoTrait, db::Database};
use async_trait::async_trait;
use double_map::DHashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone, Serialize, Deserialize)]
pub struct Host {
    pub host_id: HostId,
    pub host_label: String,
    pub host_location: String,
}

pub struct HostsDao {
    database: Arc<Mutex<Database>>,
    hosts: DHashMap<HostId, String, Box<Host>>,
}

#[async_trait]
impl DaoTrait<Host> for HostsDao {
    async fn new(db: Arc<Mutex<Database>>) -> HostsDao {
        let mut hosts = DHashMap::<HostId, String, Box<Host>>::new();

        let found_hosts = db
            .lock()
            .await
            .query("select host_id, host_label, host_location from host;", &[])
            .await
            .unwrap_or_else(|_| panic!("Fetching host data failed!"))
            .iter()
            .map(|row| Host {
                host_id: row.get("host_id"),
                host_label: row.get("host_label"),
                host_location: row.get("host_location"),
            })
            .collect::<Vec<Host>>();

        for host in found_hosts {
            hosts.insert(host.host_id, host.host_label.clone(), Box::new(host));
        }

        HostsDao {
            database: db,
            hosts,
        }
    }

    async fn get_all(&self) -> Vec<&Host> {
        self.hosts
            .values()
            .map(std::convert::AsRef::as_ref)
            .collect::<Vec<&Host>>()
    }

    async fn get_all_mut(&mut self) -> Vec<&mut Host> {
        self.hosts
            .values_mut()
            .map(std::convert::AsMut::as_mut)
            .collect::<Vec<&mut Host>>()
    }

    async fn get_one_by_id(&self, host_id: &HostId) -> Option<&Host> {
        Some(self.hosts.get_key1(host_id)?.as_ref())
    }

    async fn get_one_by_id_mut(&mut self, host_id: &HostId) -> Option<&mut Host> {
        Some(self.hosts.get_mut_key1(host_id)?.as_mut())
    }

    async fn get_one_by_name(&self, host_name: &str) -> Option<&Host> {
        Some(self.hosts.get_key2(host_name)?.as_ref())
    }

    async fn get_one_by_name_mut(&mut self, host_name: &str) -> Option<&mut Host> {
        Some(self.hosts.get_mut_key2(host_name)?.as_mut())
    }
}
