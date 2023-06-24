pub use super::lib::Id as HostId;
use crate::dao::db::Database;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

#[derive(Clone, Serialize, Deserialize)]
pub struct Host {
    pub host_id: HostId,
    pub host_label: String,
    pub host_location: String,
}

pub struct HostsDao {
    database: Arc<Mutex<Database>>,
    hosts_by_id: HashMap<HostId, Host>,
    hosts_by_label: HashMap<String, Host>,
}

impl HostsDao {
    pub async fn new(db: Arc<Mutex<Database>>) -> HostsDao {
        let mut hosts_by_id = HashMap::new();
        let mut hosts_by_label = HashMap::new();

        let hosts = db
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

        for host in hosts {
            hosts_by_id.insert(host.host_id.clone(), host.clone());
            hosts_by_label.insert(host.host_label.clone(), host.clone());
        }

        HostsDao {
            database: db,
            hosts_by_id,
            hosts_by_label,
        }
    }

    pub fn get_host_by_id(&self, host_id: HostId) -> Option<Host> {
        self.get_host_by_id(host_id).clone()
    }

    pub fn get_host_by_name(&self, host_name: String) -> Option<Host> {
        self.get_host_by_name(host_name).clone()
    }

    pub fn get_all_hosts(&self) -> Vec<Host> {
        self.hosts_by_id
            .values()
            .map(|x| x.clone())
            .collect::<Vec<Host>>()
    }

    pub fn register_new_host(
        &mut self,
        host_name: String,
        host_location: String,
    ) -> Result<(), tokio_postgres::Error> {
        Ok(())
    }
}
