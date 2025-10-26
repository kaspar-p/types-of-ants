# ant-just-checking-in

The canaries, monitoring, continuous health testing software.

## Requirements

Each test needs to register itself somehow in the database. Each test will emit
data about their passing/failures, and potentially additional debug logs into
the database.

All of this data may be logged into different databases. The data should only be
kept for the last month, anything older than that can be thrown away.

All functionality that the project requires is tested here. This includes:

### ant-on-the-web

- The site is up, contains some ants (`ping` or `curl -L` test)
- Suggestions are received and listed in the site afterwards
- New emails are received and confirmation emails are sent to them.
- Each page is working as expected and is populated with the relevant data.

### ant-building-projects

- projects can be spun up on machines they didn't exist on before (deploying for
  the first time)
- projects can be updated after they already exist (deploying after the first
  time)

## Details

`ant-just-checking-in` will probably be written in Rust, and will commit the
data into a database. The data will need to include the following:

- A unique string ID, which is the test name
- The timestamp the test was performed
- The status of the test (pass/fail)
