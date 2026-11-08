# ant-backing-it-up

Takes hourly backups of databases and stores them in `ant-archive`.

## Backups

A loop in `main.rs` calls `POST /backup` once per hour for each configured
project (only `ant-data-farm` today). Each backup:

1. Runs `pg_dump` to a local file in the persistence directory.
2. Uploads it to ant-archive, in the `b-typesofants` bucket under
   `ant-backing-it-up/backups/{project}/`.
3. Deletes the local file.
4. Rotates that project's backups.

## Rotation

Rotation keeps the newest backup in each of the most recent periods that have
a backup, and deletes the rest:

| Period | Kept |
| --- | --- |
| hour | 6 |
| 6 hours | 4 |
| day | 7 |
| week | 5 |
| calendar month | 6 |

That's at most 28 backups, about 21 in steady state with hourly backups.
Periods are fixed windows of UTC time, so a backup that rotation deletes is
never one a later run would have kept, and if backups stop being taken, no
backups are rotated away.

`POST /clean` with `{"sourceProject": "ant-data-farm"}` runs rotation on its
own.

Deleting a backup only marks it deleted in ant-archive. The space comes back
when the ant-archive vacuum purges it.

## Retired: ant-fs and ant-backing-it-up-db

Backups used to be zipped, encrypted, sent to an `ant-fs` instance, and
recorded in the `ant-backing-it-up-db` database. That path is retired. The
`fetch-backup` binary still reads from it, to restore backups taken before the
move to ant-archive.
