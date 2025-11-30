# ant-host-agent

The on-host agent for typesofants. Controls new service installations and
deployments.

## Deployments

Deployments happen in 3 phases: build, install, and deploy. The build phase is
entirely local and produces a known .tar format, see
[docs/deployment-manifest.md](../../docs/design/deployment-manifest.md) for the
structure of that file. The deployment manifest is placed within the
ant-host-agent's store at a particular location (this is done manually, since
ant-host-agent has no APIs for consuming large files and saving them locally).

The deployment .tar is unpacked and docker image/tags registered once
installation is done, via the `POST /service/service-installation` API. This is
_harmless_, and can be done at any time. There is no downside to doing this
immediately on every build, and cannot affect the runtime of other services
(unless they are nearly out of disk space and replicating the binary fills the
disk...)

The final phase is deployment, where the newly installed service directory is
pointed-to and declared fit for use. This is done with the
`POST /service/service` API. This is meant to be a quick operation to facilitate
rollbacks, where multiple versions of a piece of software can be installed, but
only one used at a time. Of course, persistent data must be both backwards- and
forwards-compatible for rollbacks to work correctly.

## Secrets management

Secrets are stored in ant-host-agent persistent storage. There are APIs to write
new secrets and delete existing secrets:

- `POST /secret/secret`: create a new secret, overwriting the secret if it
  already exists.
- `DELETE /secret/secret`: delete a secret if it exists. Succeeds even if it
  doesn't.
- `GET /secret/secret`: _peek_ at a secret, detecting if it's there. Does not
  return the secret key material.

There are no APIs to get the secret key material.
