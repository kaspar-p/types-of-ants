# Adding a remote `ant-archive-storage` service

This is only necessary if they aren't running within typesofants infrastructure,
and therefore don't have:

1. A local `ant-matchmaker` Consul agent to let them be discoverable
2. A local `ant-host-agent` deployment agent.

## Step: Register with Consul

First, get the hostname + port that this project should be accessible at. Write
that into a JSON file like:

```json
{
  "Datacenter": "dc1",
  "Node": "node-display-name-here",
  "Address": "hostname.ant-archive-node-here.com",
  "Service": {
    "ID": "ant-archive-storage-on-xyz",
    "Service": "ant-archive-storage",
    "Port": 443
  }
}
```

> NOTE: it's important that that `Node` here matches the secret-prefix in the
> client secrets, which is structured as
>
> ```txt
> node:username:password
> node2:username:password
> ```
>
> So it's important that it's `"Node": "node"` or `"Node": "node2"`.

Then, call Consul's Catalog Register Service API from local:

```bash
ah curl localhost:9990/v1/catalog/register -X PUT --data @file.json
```

This replicates the state everywhere. Ensure it works by re-querying for "all
nodes of the service"

```bash
ah curl ant-matchmaker:prod/v1/catalog/services
```

## Step: Enable in `ant-archive-db`

The dynamic set of nodes is not useful for the persistence layer, where we have
to be certain which node an object is on, so there's the table
`archive_storage_client` that holds this information.

If not already there, create a row:

```sql
BEGIN;

insert into archive_storage_client
  (host_id, is_active, capacity_bytes, protocol)
values
  ('node-display-name-here', true, 0, 'https')
;

COMMIT;
```

Where the `host_id` does need to match what `Node` is configured as, in Consul.
The capacity bytes initially being set to zero means that this node will not
qualify for any object placement. Setting `is_active=false` would achieve the
same thing.

## Step: Test with object-specific placement
