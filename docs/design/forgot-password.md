# Forgot Password

To reset a forgotten password, the user first requests that page. They type in
their username and phone number. If that combination exists in the database, we
sent then a one-time code.

If they enter that one-time code successfully, they are given a JWT by the
`POST /api/users/password-reset` route. The JWT expires quickly, is signed on
the server, and they are meant to use it to submit their new passwords.

Another request to `/POST /api/users/password` requires that JWT to be contained
in the request, so that the users new password can be applied and we can be sure
the secret means they were allowed to do that.
