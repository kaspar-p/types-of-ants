# Moving a project

This is the notes / instructions taken during 2026-09-20 movement of
`ant-backing-it-up` from `antworker001` (where all the webserver stuff is) to
`antworker003`, where not much runs.

For stateless services, it's like:

1. Add new service, deploy it.
2. Migrate data if there is any.
3. Deactivate old service.

For batch services that can miss a run, it's like:

1. Add new service.
2. Migrate data if there is any to new host.
3. Deactivate old service.
4. Deploy new service.

## Step 1: Add the new `services.json` entry

The deployment system needs to understand where to deploy the new project.

## Step 2: Remove the old `services.json` entry

But remember which host it was on, to recover its persistence directory at some
point.

## Step 3: Redeploy

Run `ah deploy <project>` to get it deployed. This will then have TWO concurrent
instances running, ensure that's fine...

## Step 4: Migrate the data

Recall the host from Step 2. SSH into it and grab the data.

Move the data from the persistence directory from one host to another. No easy
tool exists for this atm.

## Step 4: Get onto the previous host and deactivate the service

Recall the host from Step 2. SSH into it and `ant-host-agent` has an API for
deactivating the service:

```bash
$ curl -X DELETE localhost:3232/service/service -H 'content-type: application/json' -d '{"service_id":"SERVICE"}'
Service disabled.
```
