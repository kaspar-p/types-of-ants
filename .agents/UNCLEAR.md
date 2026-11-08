# Unclear items

Things the agent can't find out from the repo: production data (each a
candidate for an MCP tool or another way of getting access), and open
questions about the system. Facts go in `DESIGN.md` instead.

## Production state

- [ ] Logs from past `ant-backing-it-up` runs: do backups succeed, and how long do they take?
- [ ] Which host is `ant-backing-it-up` running on right now? Was the old instance on `antworker001` deactivated, as `docs/moving-a-project.md` describes?
- [ ] Is the old instance's persistence directory still on `antworker001`, and what's in it?

## Stored data

- [ ] What's currently in the `b-typesofants` bucket under `ant-backing-it-up/backups/`: how many objects, how big, how old? The first rotation run would delete everything outside the retention set.
- [ ] How many objects do ant-archive buckets hold? This decides whether `list_objects` needs pagination.
- [ ] Are there still backups in `ant-fs`, and in the `ant-backing-it-up-db` table, that aren't in ant-archive?
- [ ] Free and used capacity on each ant-archive storage node.

## Usage

- [ ] Access logs for `GET /backups` on `ant-backing-it-up`: did anything still call it before it was removed?
- [ ] Which clients call ant-archive, and which endpoints do they use?
- [ ] Consul logs or incidents behind adding `server_rejoin_age_max = "1440h"`.

## Open questions about the system

- [ ] When is a trait with an in-memory fake enough, and when is a real-service integration test required?
