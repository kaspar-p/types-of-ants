use std::ffi::OsStr;

use sysinfo::System;

use crate::common::kill_project::{KillProjectRequest, KillProjectResponse, KillStatus};

pub enum KillProjectError {
    NothingToKill,
    FailedToKill,
}

pub async fn kill_project(
    KillProjectRequest { project }: KillProjectRequest,
) -> Result<KillProjectResponse, KillProjectError> {
    let mut sys = System::new_all();
    sys.refresh_all();

    let o_proc = sys
        .processes_by_exact_name(OsStr::new(project.as_str()))
        .last();
    let proc = match o_proc {
        None => return Err(KillProjectError::NothingToKill),
        Some(proc) => proc,
    };

    if proc.kill() {
        return Ok(KillProjectResponse {
            status: KillStatus::Successful,
        });
    }

    Err(KillProjectError::FailedToKill)
}
