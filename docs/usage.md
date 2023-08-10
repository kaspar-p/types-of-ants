# How to add new ants

1. Run `./automation/build`, which builds the `add` binary in the `cli` folder. This is the CLI that makes it easy to add new ants.
2. Run the `./cli/add` binary, and add all the ants. Type `.done` when finished adding ants, it will randomly put them into `ants.txt`, the ants source of truth.
3. Run the `./generate.sh` file. It will update the right files.
4. Commit via `git add .` and commit all of those changes, title it like "new ants release" or something exciting.
5. Then, run the `./generate.sh` script again. This is going to pick up on the new git history and increment the version number on the website. It should probably be smarter than this, but it isn't.
6. Add those changes via `git add .` and commit then again, that's it!
