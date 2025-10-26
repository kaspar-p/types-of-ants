# Authentication Levels

There are 3 levels of authentication that a human user goes through to
authenticate with the services.

## Unauthenticated

Some routes must be unauthenticated, since they are the entrypoint for the user.
For example, `POST /api/users/signup` is unauthenticated because users who have
never signed up before will not be authenticated. Most data-only routes are also
unauthenticated, like listing the ants for the main page, the feed, and so on.

## Weak Authentication

typesofants.org _requires_ that accounts sign in via 2FA. To facilitate this,
after a user has created a username/password combination, they are authenticated
(via a cookie) with "weak authentication". There are certain routes that are now
available that were previously unavailable, but most authenticated routes are
still unavailable.

This is to represent the "state machine" of authentication, where a user first
enters their username/password combination, and then must enter their 2FA key.
These are 2 different steps, and to make it easier for users to reload the page,
..., they are given a cookie for this state.

Routes that have optional authentication, like suggesting a new ant, will not
consider weak authentication as authentication at all, and will fallback to
unauthenticated access if only weak authentication is present.

Weak authentication is sensitive, and should only be handed out by a select few
APIs (e.g. `POST /api/users/login`), and consumed by others (2FA related
routes).

## Strong Authentication

Most routes are this state, it requires the user to be entirely logged in
(including 2FA) to be accessed. Sensitive privileged operations like changing
your username require strong authentication.

Strong authentication JWTs should _only_ be given out by the routes handling 2FA
verification. They can be widely accepted, though.
