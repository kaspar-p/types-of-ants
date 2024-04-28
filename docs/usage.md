# How to add new ants

1. Run `cd static_site && make`, which builds the `add` binary in the `cli`
   folder. This is the CLI that makes it easy to add new ants.
2. Run the `./add` binary, and add all the ants. Type `.done` when finished
   adding ants, it will randomly put them into `ants.txt`, the ants source of
   truth.
3. Run the `./bin/generate_site` file. It will update the right files. Make sure
   to get the arguments right!
4. Commit via `git add .` and commit all of those changes, title it like "new
   ants release" or something exciting! You're done!
