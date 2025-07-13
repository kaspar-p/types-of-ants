# Promoting beta.typesofants.org to serve content from typesofants.org

Documenting the steps performed to promote beta to production.

## Steps I think

### Precursors

1. Change database credentials for prod and beta so no mistakes! happen.

### Move beta

1. Deploy `ant-gateway` with `beta.typesofants.org` on beta machine, 002.
   1. Expose 002 machine `ant-gateway` ports on network.
1. Deploy `ant-data-farm` on beta machine, 002.
1. Deploy `ant-on-the-web` on beta machine, 002, pointing to 002 for db.

### Deploy prod

1. Deploy `ant-on-the-web` on prod ws machine, 000.
1. Deploy `ant-gateway` with `typesofants.org` on prod gateway machine, 001.
   1. Should already be port-forwarding and exposed.

### Deploying `ant-gateway` to 002

1. Log on
1.
