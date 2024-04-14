# How to add new ants

1. Run `./automation/build.sh`, which builds the `add` binary in the `cli`
   folder. This is the CLI that makes it easy to add new ants.
2. Run the `./cli/add` binary, and add all the ants. Type `.done` when finished
   adding ants, it will randomly put them into `ants.txt`, the ants source of
   truth.
3. Run the `./generate.sh` file. It will update the right files.
4. Commit via `git add .` and commit all of those changes, title it like "new
   ants release" or something exciting! You're done!
