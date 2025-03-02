# Make a release

## Step 1: add new ants

Start from the root of the Git repository.

1. Run `cd static_site && make`, which builds the `add` binary in the `cli`
   folder. This is the CLI that makes it easy to add new ants.
2. Run the `./add` binary, and add all the ants. Type `.done` when finished
   adding ants, it will randomly put them into `ants.txt`, the ants source of
   truth.
3. Run the `./bin/generate_site` file. It will update the right files. Make sure
   to get the arguments right!
4. Commit via `git add .` and commit all of those changes, title it like "new
   ants release" or something exciting! You're done!

## Step 2: Make a SQL migration file

Start from the root of the Git repository.

1. Run the `release_to_sql.ts` script:

```bash
npx tsx projects/ant-data-farm/data/src/release_to_sql.ts ./static_site/releases/<release-file-name> <release-number>
```

> 1. The release file name is the one generated in step 1.
> 2. The release number is 1 higher than the latest release number. If there is
>    no recent `migrations/` file that shows this (there probably is), see the
>    [README](./README.md) for info on how to log into the DB and make the query
>    yourself.

1. Copy this output from the terminal (or pipe) into a new file in
   `./projects/ant-data-farm/data/sql/migrations`.

## Step 3: Deploy the changes into the database

Using the transaction generated, we need those changes in the database.

1. Log onto the production database host, see the [README](./README.md) for
   more.
2. Copy-paste the migrations needed in the database, by the number. For example,
   a migration file beginning with `09` will be DB migration item 9.
