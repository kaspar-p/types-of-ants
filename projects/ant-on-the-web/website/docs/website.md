# ant-on-the-web

`ant-on-the-web` is the name for the website portion of the project. The technology with which it will be written is not yet known, but some flavor of Typescript with a corresponding framework.

## Goals

Ant on the web is the dashboarding area for the information/data the other parts of the service will generate. These different types of data will be shown at various routes.

The valid routes for the website will be:

- The main site (`typesofants.org`)
- Suggestions (`typesofants.org/suggestions`)
- Test suite passing/failing statuses (`typesofants.org/tests`)
- Software deployments (`typesofants.org/deployments`)
- Provisioning and host information (`typesofants.org/hosts`)
- Blog (`typesofants.org/blog`)
- Contact me/Information page (`typesofants.org/info`)
- [maybe] Read-only database and query submitter (`typesofants.org/data`)

### Homepage (`typesofants.org`)

It should look largely similar to how it does today, except perhaps updated interfaces for the suggestions and email lists.

The main big different would either be a top-bar or hamburger menu that would have links to the other parts of the site.

### Suggestions (`typesofants.org/suggestions`)

The user should be able to see a paginated list of suggestions here. They should be able to filter suggestions based on various criteria:

- the suggestion status
- their favorites
- the entries of a certain user

Similar to how the Github Issues page looks today, clicking on a suggestion should bring that suggestion up with corresponding metadata. Either on a separate page or in a modal or something. The metadata shown should be:

- The username of the user that entered the suggestion
- The status of the suggestion
- The content of the suggestion
- Any sort of suggestion edit history
- The history of acceptance, closure, etc.

The features listed here all require API support.

### Tests (`typesofants.org/tests`)

Goal: the user should be able to see all of the tests, up to a certain history, filtered/formatted in ways that are helpful for them. The goal is to allow the user to see into the database, without knowing SQL.

The different dimensions that tests have are per-host, per-project, and per-test. For example, a user might be interested in:

- the tests that affect a certain host, to determine if a certain host is failing
- the tests that affect a certain of project (`ant-on-the-web`, for example) across many hosts, to determine if a project had a bad code push
- a single test on all hosts, to determine if that test is flaky or written incorrectly.
- the tests that affect a single host, in a single project, to determine if a certain architecture is incompatible with the project.
- A single test, across all hosts, to determine if that test is written incorrectly or makes some false assumptions,
- All other combinations of tests, hosts, and projects.

The tests should also be historical, with appropriate line-graph visualizations. It's not very useful to just see the moment-to-moment test status. Who knows if, 5 minutes ago, everything was broken?

The test history should be available at least a month back. There may be a difference here in what the database stores, with what the UI shows. The database may keep test data for much longer. Or maybe not, might be too expensive. We'll see.

### Deployments (`typesofants.org/deployments`)

Goal: the user should be able to look into the flow of software deployments. It should be transparent for everything going on. Rather than seeing "what is the most recent version", the deployments should be about recency, environments, and status.

The UI should be simple, and should show the flow of deployments from development, to build, to beta, to prod, with timestamps of each status, and potentially the status of each deployment on each host.

For the beginning, perhaps a single deployment flow (for the single host), is enough.

### Hosts (`typesofants.org/hosts`)

Goal: to let the user see which hosts are running which projects, and their current health status (test snapshot).

Should probably just be a grid of information, each cell corresponding to a host. That is, until there is greater hierarchy, like a fleet of hosts that only does `ant-on-the-web`, or a fleet that only runs `ant-just-checking-in`, rather than hosts that run everything.

### Blog (`typesofants.org/blog`)

A blog!

### Contact me (`typesofants.org/info`)

A page for the Github, the code, my email, etc.

### ['maybe'] Database (`typesofants.org/data`)

Potentially show the users a read-only version of the database that they can run their own SQL queries on. Not sure. Sounds insecure.
