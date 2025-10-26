# Deployment Manifest

The "deployment manifest" is the design of the file(s) used to represent a
deployment on a machine. The machine receiving this file should expect that it
is in this specified format, and the machine creating this file should adhere to
this format during building.

## Files

The deployment manifest is a single file, a tar file containing the necessary
deployment files. The tar file must be a directory containing at least 1 file, a
systemd unit file called `[project].service`, where `[project]` is the name of
the project being deployed, e.g. `ant-data-farm`.

## The `deployment.tar` file

The file is a compressed directory containing everything the deployment requires
to begin successfully, _except credential material_. They should be entirely
fine to send between machines, store un-encrypted, and so on.

The smallest version of a `deployment.tar` is a single file, the
`[project].service` systemd service file. If the deployment is a one-shot
request or command, this is enough for that.

This is not enough for most deployments, and the next-smallest is the systemd
unit file along with a binary that systemd is meant to run.

## Deployment semantics

All deployments can be performed by un-tarring the required file and rotating
the systemd service. The systemd service is always named `[project].service`
(e.g. `ant-data-farm.service`), no matter the type of project.

If the service requires any secrets, those are listed by-name in the
`manifest.json` `secrets` field. The deployment MUST mount them as a `/secrets`
directory within the un-tarred `deployment.tar` file.

For example, a common `deployment.tar` for `ant-on-the-web` might contain:

```ls
deployment.tar
  .env                        (non-sensitive environment configuration)
  ant-on-the-web              (binary)
  ant-on-the-web.service      (systemd unit file)
  static/                     (static web HTML/JS/CSS assets to serve)
    ...
```

and then after unpacking:

```ls
install-directory/
  .env                        (non-sensitive environment configuration)
  ant-on-the-web              (binary)
  ant-on-the-web.service      (systemd unit file)
  static/                     (static web HTML/JS/CSS assets to serve)
    ...
  secrets/
    jwt.secret                (secret: the JWT signing key)
    ...
```

## Docker Images

Docker images need to be registered separately from binaries or timer-related
systemd unit files, with the Docker daemon running on the host.

Docker images must be named `docker-image.tar` within the unpacked directory of
the `deployment.tar` (two-levels of .tar), and will be loaded via the regular:

```bash
docker load --input ./docker-image.tar
```

command before rotating the systemd service.
