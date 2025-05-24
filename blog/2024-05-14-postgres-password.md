# Postgres Password

> May 14, 2024

Today I learned that setting a different password on a Postgres docker container
(even one with a mounted volume!) essentially created an entirely new database.

It will apply some data, but no schema migrations, and all past data will
probably be lost forever until you revert it back, and then you have two
histories. I'm not sure where that data goes.
