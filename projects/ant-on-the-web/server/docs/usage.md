# Usage

## How to add new ants

1. Navigate to this project from the monorepo root, `cd ant-on-the-web`.
2. Build the CLI tool with `go build` in the `cli` repository.
3. Run the created `add` binary with `./add`.
4. Add the ants you want to add, make sure to type `.done` rather than an ant
   name to finish.
5. Run `./generate.sh` to generate the website and README.md based on this list
   of ants.
6. Add and commit these changes.
7. Then, run `./generate.sh` again. This is to pick up the new commit and add
   the ants into the banner with a `diff` tool.
8. Add and commit these changes too.
9. Open `index.html` in the browser to make sure everything looks fine.
10. Finally, you can `git push` everything to `main` to publish your new
    changes.

## How to compile and run the webserver

There are two different docker images that are used to support the web server.
The first is the nginx based webserver that actually serves the static content
that we want to serve, and the second is the `certbot`, which updates the SSL
certificates.

Currently, the webserver is only running beta.typesofants.org, not the live
typesofants.org.

There is a single compilation step: `docker-compose build`. Then, to run the
webserver and certbot, use `docker-compose up`. These should be the only two
commands you need after checking out the code to get the site running!
