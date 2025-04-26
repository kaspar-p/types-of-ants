# ant-naming-domains

Owns the `ddclient` instance on one of the hosts that is responsible for
updating CloudFlare IP mappings when the public IP of the physical location that
typesofants is deployed changes.

## Installing and running

This is a project managed via `docker-compose` and `systemctl`.

Install the latest version on the host:

```bash
./scripts/install-docker-service.sh ant-naming-domains
```

which will spit out the version, something like:

```txt
INFO [ 2025-04-26T21:06:05+00:00 ant@antworker002 types-of-ants ] INSTALLED [ant-naming-domains] VERSION [2025-04-26-21-06-5d82aea]
INFO [ 2025-04-26T21:06:05+00:00 ant@antworker002 types-of-ants ]   when:        2025-04-26T21:06:05+00:00
INFO [ 2025-04-26T21:06:05+00:00 ant@antworker002 types-of-ants ]   install dir: /home/ant/service/ant-naming-domains/2025-04-26-21-06-5d82aea
INFO [ 2025-04-26T21:06:05+00:00 ant@antworker002 types-of-ants ]   version:     2025-04-26-21-06-5d82aea
INFO [ 2025-04-26T21:06:05+00:00 ant@antworker002 types-of-ants ]   unit file:   /home/ant/service/ant-naming-domains/2025-04-26-21-06-5d82aea/ant-naming-domains.service
ant@antworker002:~/types-of-ants$ ./scripts/deploy-systemd.sh ant-naming-domains 2025-04-26-21-06-5d82aea
```

Taking the version `2025-04-26-21-06-5d82aea`, run:

```bash
./scripts/deploy-systemd.sh ant-naming-domains 2025-04-26-21-06-5d82aea
```

And it should be started. You can be sure by running `docker ps` and seeing a
`ant-naming-domains` container, or
`sudo journalctl -u ant-naming-domains.service --since '1 hour ago'`.
