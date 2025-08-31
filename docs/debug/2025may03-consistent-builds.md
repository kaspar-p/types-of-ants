# Consistent builds

I wasn't aware of this, but NextJS adds a build hash randomly when it runs
`next build`, which runs based on our `npm build`.

This is generated randomly by default, but apparently they allow an override
[generateBuildId](https://nextjs.org/docs/app/api-reference/config/next-config-js/generateBuildId)
function. This became an issue when `ant-on-the-web` was built for multiple
machines. The machines would name their files something like

```txt
main-hashA.js
main-hashB.js
main-hashC.js
```

and since the requests were routed round-robin, the `GET /` request would return
an `index.html` that was precompiled to ask for one of `main-hashA.js`,
`main-hashB.js`, or `main-hashC.js`. But due to the round-robin, it was
effectively guaranteed to contact a _different machine_ for that request.
Meaning the `GET /` would be geared for `hashA`, but would get routed by NGINX
to the `hashB`.

Every request got a 404.

The _right_ solution is to use a single machine to build and distribute the
installation, but that's still not perfect. Different processor architectures
necessitates different machines, and if they want to be running the same
project, it still needs to be deterministic.

I'll probably use the `GIT_HASH` that NextJS recommends, or the
`INSTALL_VERSION` that we send.
