# Design of typesofants.org

I want to transform typesofants.org into more of an interactive developing website. I want everything that normally goes into developing a service to be visible to everyone.

## Goals

There are many processes that businesses running to make money would never want their customers (or competing businesses) to see. These include things like deployment schdules for software, versioning info, architecture diagrams and documents, currently passing or failing tests, and others. The goal for typesofants.org is to be a full end-to-end "service", but show all of those things off as a part of the development process.

The things I want to show off should be paths off the main site. Currently, only the main site is a path. The paths I want will be:

- The main site (`typesofants.org`)
- Suggestions (`typesofants.org/suggestions`)
- Test suite passing/failing statuses (`typesofants.org/tests`)
- Software deployments (`typesofants.org/deployments`)
- Blog (`typesofants.org/blog`)
- Contact me/Information page (`typesofants.org/info`)

## High-level Architecture

The architecture of typesofants.org will be much different than it is right now. Currently, it's a single static HTML site, hosted on Github.

### Repository

Everything regarding this project will stay in this Github repository, exactly the same. The structure of the repo will change dramatically. There are at least three separate packages, each will be a top-level directory.

### Website

The website will have two aspects, `typesofants.org` and `beta.typesofants.org`. They both will work nearly exactly the same, except for the data they fetch will come from a different source. One will come from a beta database, one will come from a prod database.

The name for the top-level directory is not yet known. Candidates are:

- ant but it's a website (abw)
- ant in a web / ant in the web (aiaw/aitw)
- more?

Details on the structure and architecture of the website will be included in a separate design doc.

### Canaries, Monitoring, Testing

There has to be software that is continuously monitoring the site, making sure everything is working correctly. It will essentially be a suped up CRON job, running once every 5 minutes to test the sites capabilities.

The name for this top-level directory will be `ant-just-checking-in`.

Details on the structure and architecture of `ant-just-checking-in` will be included in a separate design doc.

### Deployments

When software is updated, that new software has to be pushed into the relevant places and made live. Currently, this is done automatically by Github through the site being hosted by them. The deployments will need to emit tracking data, for the site to pick up and visualize in some way on `typesofants.org/deployments`.

The deployments will really be a glorified event-based worker. New events in the form of software updates will trigger workflows. Those workflows will replace the software, start the server, etc.

Ashley had the good idea of having a little graphic of an ant building, so the name is going to be `ant-building-projects`. That's the top-level directory, too.

Details on the structure and architecture of `ant-building-projects` will be included in a separate design doc.

# ant-just-checking-in

The canaries, monitoring, continuous health testing software.

## Requirements

Each test needs to register itself somehow in the database. Each test will emit data about their passing/failures, and potentially additional debug logs into the database.

All of this data may be logged into different databases. The data should only be kept for the last month, anything older than that can be thrown away.

All functionality that the project requires is tested here. This includes:

**ant-in-the-web**

- The site is up, contains some ants (`ping` or `curl -L` test)
- Suggestions are received and listed in the site afterwards
- New emails are received and confirmation emails are sent to them.
- Each page is working as expected and is populated with the relevant data.

**ant-building-projects**

- projects can be spun up on machines they didn't exist on before (deploying for the first time)
- projects can be updated after they already exist (deploying after the first time)

## Details

`ant-just-checking-in` will probably be written in Rust, and will commit the data into a database. The data will need to include the following:

- A unique string ID, which is the test name
- The timestamp the test was performed
- The status of the test (pass/fail)

# ant-building-projects

The deployment and building software.

## Requirements

It needs to be able to know if a project has changed. This can either be a push or pull method. Push would require the existence of an API, and pull would require some sort of CRON job cycle.

It then needs to pull the new changes, rebuild the project, stop the current program, and restart the program.

Building the project will be handled by the build system, like Bazel. Everything else is straightforward.

Deployment data will also need to be logged, to show on /deployments. Each step of the deployment will need to emit that data. The data includes:

- Timestamp of the start of the step
- Timestamp of the end of the step
- The version that it is upgrading from
- The version it is upgrading to
- The machine that it is deploying onto
- The project that is currently being deployed
