# May 23-25, 2024 Non-tweeting event

## What happened

Logging onto the host the logs for the tweet agent looked fine.

Was unable to `docker ps` and logging into the DB was with the `typesofants`
password, the old database. No idea why that happens sometimes.

Docker kept saying "could not connect to daemon" and the
`sudo journalctl -u ant-data-farm.service` had the same error, failing to start.

Doing `sudo journalctl -xef` gets me logs like:

```log
May 25 10:58:54 antworker000 docker.dockerd[10020]: ERROR: ld.so: object '/usr/lib/arm-linux-gnueabihf/libarmmem-${PLATFORM}.so' from /etc/ld.so.preload cannot be preloaded (cannot open shared object file): ignored.
May 25 10:58:54 antworker000 docker.dockerd[10073]: ERROR: ld.so: object '/usr/lib/arm-linux-gnueabihf/libarmmem-${PLATFORM}.so' from /etc/ld.so.preload cannot be preloaded (cannot open shared object file): ignored.
May 25 10:58:55 antworker000 docker.dockerd[10073]: time="2024-05-25T10:58:55.244620899-04:00" level=error msg="failed to initialize a tracing processor \"otlp\"" error="no OpenTelemetry endpoint: skip plugin"
May 25 10:58:56 antworker000 docker.dockerd[10020]: failed to start daemon: Error initializing network controller: error obtaining controller instance: failed to create NAT chain DOCKER: Could not create nat/DOCKER chain: ERROR: ld.so: object '/usr/lib/arm-linux-gnueabihf/libarmmem-${PLATFORM}.so' from /etc/ld.so.preload cannot be preloaded (cannot open shared object file): ignored.
```

From
<https://stackoverflow.com/questions/75713844/how-to-resolve-failed-to-create-nat-chain-docker-as-reboot-not-working>
trying to `sudo apt update && sudo apt upgrade`. Had 380MB of changes, maybe
this was needed.

Going to reboot after this.

Also changed the `ssh2ant` to `dig +short` first, before resolving. This seems
to be more consistent, since letting `ssh` resolve the "fake" IP hung forever it
seems.

That didn't work, same error in the logs.

After a _really_ _really_ long time, it turned out to be conflicting
installations of docker. I'd installed on this machine (antworker 000, the
Raspberry Pi), via `apt-get` forever ago, but recently my installation guide
listed `snap`, so there were two. Purging everything and just getting the `snap`
one working took a while.

After that, the default MV of PostgreSQL had gone from 16 to 15 via `snap`
default installation. Needed to target 15 via the docker image.

## Unknowns

- Why is the old DB with old password still around?
  - How can I get rid of it, or destroy postgres on this machine?
- Why did the database restart and fail to come back up?
