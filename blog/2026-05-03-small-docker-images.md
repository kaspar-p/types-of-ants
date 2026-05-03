# Small Docker Images

<https://typesofants.org> includes software for manipulating Docker images,
mostly around it's CI/CD and build systems.

There are a suite of integration/functional tests over the codebase that test
things like the progression of the deployment and unpacking/repacking .tar
files, including the automatic `docker-image.tar` that's included in each build
bundle if the project wants one.

Those tests always historically used a production image, usually the NGINX
reverse proxy `ant-gateway`, as their source of tests. It was a fact of life
that those tests were slow, they were doing real work after all!

However, the unpacking-repacking behavior of the API means that any large files
need to be carted around. To make matters worse, I'd tried to make my life
better by storing "test archives", prebuilt archives that had a certain
structure:

```fs
test-archives/
  test-malformed-inner-file.tar.gz
  test-happy-path.tar.gz
  ...
```

but each time those files needed _changing_, I'd break out the `tar -xvf` and be
left with a horrible Git diff. I finally realized I could store just the entire
test-archives _as directories_, and at runtime package them. This is
compute-wasteful, but makes iterating way easier!

```fs
test-archives/
  test-malformed-inner-file/
    inner-file.bork
  test-happy-path/
    ...
  ...
```

This all highlighted that my tests were taking _minutes_ shipping around
~10-30MB rust binaries (`ant-host-agent`, usually), and 100-200MB docker images.

## Bring `scratch` back?

First, I tried just `trunc`ing the files and leaving them empty. This worked for
replacing `ant-host-agent` (10MB win!), but didn't work for any test touching
the Docker daemon, since it would puke on trying to read or load those empty
"images". I needed the tiniest one.

Docker has a 0-byte image named `scratch`, but it can't be pulled, saved, or
stored, it's just a keyword. Stupid.

I wanted to use `tianon/true` (124 bytes!) but it didn't come in any
Linux-flavors, so I settled for `k8s.gcr.io/pause` (288kb), which is well within
the territory of "I can't notice any slow-downs", which leaves me happy.
