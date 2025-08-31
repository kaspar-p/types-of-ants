# August 31, 2025 Outage

The ddclient was trying to set an A record for `typesofants.org`, based on the
`ANT_NAMING_DOMAINS_FQDN` variable.

However, the DNS situation from V1 to V2 had the `typesofants.org` zone
controlled by a CNAME record, pointing to `prod-v1.typesofants.org`. This was
because I then had `prod-v1.typesofants.org` pointing to `kaspar-p.github.io`
which was the statically generated site.

During the promotion process I put the `prod-v2.typesofants.org` endpoint on the
internet to do some testing and ensure nothing went wrong. There was a bug and
being able to quickly change a single record from `prod-v2` to `prod-v1` was
very helpful.

Now that V2 is live, `prod-v2.typesofants.org` is the CNAME for
`typesofants.org`.

However, ddclient never changed and was continuously trying to assert an A
record for typesofants.org, which no longer was an A record. This failed, and
when eventually the dynamic DNS changed, it just stayed down.

The fix was to switch the environment variable to changing the IP of
prod-v2.typesofants.org, but the real solution would be more monitoring, the
logs looked like:

```txt
...
INFO:   trying to set A record on typesofants.org to <ip>
FAILED: failed to set A record on typesofants.org to <ip>: no A record exists
...
```
