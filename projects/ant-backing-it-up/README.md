# ant-backing-it-up

A simple project for taking backups of databases.

This is a simple process that takes automated backups once per hour, to the DBs
it's configured to take backups of. The files aren't saved locally, they are
offloaded to a configured `ant-fs` instance.

It takes hourly backups and stores data _about_ the backups in a database, the
`ant-backing-it-up-db` database. Each backup is individually encrypted with
one-time nonces that are stored in the DB.
