use ant_archive_client::{AntArchiveClient, AntArchiveClientError};
use tracing::info;

use crate::slots::{SlotEntry, Slots};

/// Projects with backups to rotate. Each project's backups are rotated on their own.
const BACKUP_PROJECTS: [&str; 1] = ["ant-data-farm"];

#[derive(Debug, thiserror::Error)]
pub(crate) enum RotationError {
    #[error("no backups are taken for project {0:?}")]
    UnsupportedProject(String),

    #[error(transparent)]
    Archive(#[from] AntArchiveClientError),

    /// Something else deleted the backup between listing and deleting it, so another rotation
    /// may be running at the same time.
    #[error("backup {0} was listed but already deleted")]
    AlreadyDeleted(String),
}

/// Where backups are stored, so rotation can be tested without a running ant-archive.
pub(crate) trait BackupStore {
    async fn list_backups(
        &self,
        bucket: &str,
        key_prefix: &str,
    ) -> Result<Vec<SlotEntry>, AntArchiveClientError>;

    /// Returns false if there was no object at `key`.
    async fn delete_backup(&self, bucket: &str, key: &str) -> Result<bool, AntArchiveClientError>;
}

impl BackupStore for AntArchiveClient {
    async fn list_backups(
        &self,
        bucket: &str,
        key_prefix: &str,
    ) -> Result<Vec<SlotEntry>, AntArchiveClientError> {
        Ok(self
            .list_objects(bucket, key_prefix)
            .await?
            .objects
            .into_iter()
            .map(|o| SlotEntry {
                id: o.id,
                key: o.key,
                created: o.created_at,
            })
            .collect())
    }

    async fn delete_backup(&self, bucket: &str, key: &str) -> Result<bool, AntArchiveClientError> {
        self.delete(bucket, key).await
    }
}

/// Ends in a slash, so one project's prefix never matches another project whose name starts
/// with it.
pub(crate) fn backup_key_prefix(source_project: &str) -> String {
    format!("ant-backing-it-up/backups/{}/", source_project)
}

pub(crate) async fn rotate_backups(
    store: &impl BackupStore,
    bucket: &str,
    source_project: &str,
) -> Result<(), RotationError> {
    if !BACKUP_PROJECTS.contains(&source_project) {
        return Err(RotationError::UnsupportedProject(
            source_project.to_string(),
        ));
    }

    info!("Slotting backup jobs...");
    let backups = store
        .list_backups(bucket, &backup_key_prefix(source_project))
        .await?;

    let slots = Slots::new(&backups);

    info!("Filled slots:\n{}", slots.print());
    for backup in slots.unused_backups {
        info!(
            "Deleting unused backup {} ({} @ {})",
            backup.key,
            backup.id,
            backup.created.to_rfc2822()
        );

        if !store.delete_backup(bucket, &backup.key).await? {
            return Err(RotationError::AlreadyDeleted(backup.key.clone()));
        }
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use std::sync::Mutex;

    use ant_archive_client::AntArchiveClientError;
    use chrono::{DateTime, Duration, Utc};
    use pretty_assertions::assert_eq;

    use crate::{
        rotation::{rotate_backups, BackupStore, RotationError},
        slots::{SlotEntry, Slots},
    };

    /// Behaves like ant-archive: objects are listed by key prefix and deleted by key.
    struct FakeArchive {
        objects: Mutex<Vec<SlotEntry>>,
        /// Listed, but already deleted by the time rotation tries to delete them.
        stale: Vec<SlotEntry>,
    }

    impl FakeArchive {
        fn new(objects: Vec<SlotEntry>) -> Self {
            FakeArchive {
                objects: Mutex::new(objects),
                stale: vec![],
            }
        }

        fn keys(&self) -> Vec<String> {
            let mut keys: Vec<String> = self
                .objects
                .lock()
                .unwrap()
                .iter()
                .map(|o| o.key.clone())
                .collect();
            keys.sort();
            keys
        }
    }

    impl BackupStore for FakeArchive {
        async fn list_backups(
            &self,
            _bucket: &str,
            key_prefix: &str,
        ) -> Result<Vec<SlotEntry>, AntArchiveClientError> {
            Ok(self
                .objects
                .lock()
                .unwrap()
                .iter()
                .chain(self.stale.iter())
                .filter(|o| o.key.starts_with(key_prefix))
                .cloned()
                .collect())
        }

        async fn delete_backup(
            &self,
            _bucket: &str,
            key: &str,
        ) -> Result<bool, AntArchiveClientError> {
            let mut objects = self.objects.lock().unwrap();
            let before = objects.len();
            objects.retain(|o| o.key != key);
            Ok(objects.len() < before)
        }
    }

    fn t0() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    /// Hourly backups for `project`, with object ids that differ from their keys like
    /// ant-archive's do.
    fn hourly_backups(project: &str, hours: i64) -> Vec<SlotEntry> {
        (0..hours)
            .map(|h| SlotEntry {
                id: format!("object-{project}-{h}"),
                key: format!("ant-backing-it-up/backups/{project}/pgdumpbackup.{h}.bak.sql"),
                created: t0() + Duration::hours(h),
            })
            .collect()
    }

    fn kept_keys(backups: &[SlotEntry]) -> Vec<String> {
        let slots = Slots::new(backups);
        let mut keys: Vec<String> = backups
            .iter()
            .filter(|b| !slots.unused_backups.contains(b))
            .map(|b| b.key.clone())
            .collect();
        keys.sort();
        keys
    }

    #[tokio::test]
    async fn rotation_deletes_unused_backups_from_the_archive() {
        let backups = hourly_backups("ant-data-farm", 24 * 3);
        let archive = FakeArchive::new(backups.clone());

        rotate_backups(&archive, "b-typesofants", "ant-data-farm")
            .await
            .unwrap();

        assert_eq!(archive.keys(), kept_keys(&backups));
    }

    #[tokio::test]
    async fn rotation_ignores_projects_whose_names_share_a_prefix() {
        let other_project = hourly_backups("ant-data-farm-2", 24 * 3);
        let mut backups = hourly_backups("ant-data-farm", 24 * 3);
        backups.extend(other_project.clone());
        let archive = FakeArchive::new(backups);

        rotate_backups(&archive, "b-typesofants", "ant-data-farm")
            .await
            .unwrap();

        let mut other_keys: Vec<String> = other_project.iter().map(|b| b.key.clone()).collect();
        other_keys.sort();
        assert_eq!(
            archive
                .keys()
                .into_iter()
                .filter(|k| k.starts_with("ant-backing-it-up/backups/ant-data-farm-2/"))
                .collect::<Vec<_>>(),
            other_keys
        );
    }

    #[tokio::test]
    async fn rotation_fails_when_an_unused_backup_was_already_deleted() {
        let mut backups = hourly_backups("ant-data-farm", 24 * 3);
        let gone = backups.remove(0);
        let mut archive = FakeArchive::new(backups);
        archive.stale.push(gone.clone());

        let res = rotate_backups(&archive, "b-typesofants", "ant-data-farm").await;

        assert!(
            matches!(&res, Err(RotationError::AlreadyDeleted(key)) if *key == gone.key),
            "got {res:?}"
        );
    }

    #[tokio::test]
    async fn rotation_rejects_unsupported_projects() {
        let backups = hourly_backups("ant-data-farm", 24 * 3);
        let archive = FakeArchive::new(backups.clone());

        for project in ["", "ant-data", "ant-data-farm/"] {
            let res = rotate_backups(&archive, "b-typesofants", project).await;

            assert!(
                matches!(&res, Err(RotationError::UnsupportedProject(p)) if p == project),
                "project {project:?} got {res:?}"
            );
        }

        let mut all_keys: Vec<String> = backups.iter().map(|b| b.key.clone()).collect();
        all_keys.sort();
        assert_eq!(archive.keys(), all_keys);
    }
}
