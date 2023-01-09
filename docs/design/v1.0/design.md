# Design of typesofants.org

I want to transform typesofants.org into more of an interactive developing website. I want everything that normally goes into developing a service to be visible to everyone.

## Goals

The goals are put here for two reasons: as an excuse to learn the technology, and as a fun thing to showcase for the future.

There are many processes that businesses running to make money would never want their customers (or competing businesses) to see. These include things like deployment schedules for software, versioning info, architecture diagrams and documents, currently passing or failing tests, the number of servers they have and their status, and others. 

The goal of v1.0 is for typesofants.org to be a full end-to-end service, but show all of those things off, as a part of the development process.

The things I want to show off should be paths off the main site. Currently, only the main site (/) is a path. The paths I want will be:

- The main site (`typesofants.org`)
- Suggestions (`typesofants.org/suggestions`)
- Test suite passing/failing statuses (`typesofants.org/tests`)
- Software deployments (`typesofants.org/deployments`)
- Blog (`typesofants.org/blog`)
- Contact me/Information page (`typesofants.org/info`)
- Provisoning information (`typesofants.org/provisioning`)
- [maybe] Read-only database and query submitter (`typesofants.org/data`)

Beyond the website, there are other best-practices that real applications use that typesofants.org should have. In short snippets:

- Users should be able to go to `typesofants.org/provisioning` and download an executable that allows their computer to act as a web server node for the front end, like a node in a CDN. This is a fun level of interactivity for technical users. The logging could be made fun, too.
- Users should be able to see every deployment, onto every machine, and its status. That dashboard should just be public. This includes tests passing/failing.

## High-level Architecture

The architecture of typesofants.org will be much different than it is right now. Currently, it's a single static HTML site, hosted through Github Pages.

### Repository

Everything regarding this project will stay in this Github repository, exactly the same. The structure of the repo will change dramatically. There are at least three separate packages, each will be a top-level directory.

### Website

The website will have two aspects, `typesofants.org` and `beta.typesofants.org`. They both will work nearly exactly the same, except for the data they fetch will come from a different source. One will come from a `beta` database, one will come from a `prod` database.

The name for the top-level directory is not yet known. Candidates are:

- ant but it's a website (abw)
- ant in a web / ant in the web (aiaw/aitw)
- [**WINNER**] ant on the web

Details on the structure and architecture of the website will be included in a separate design doc.

### Canaries, Monitoring, Testing

There has to be software that is continuously monitoring the site, making sure everything is working correctly. It will essentially be a supercharged CRON job, running once every 5 minutes to test the sites capabilities.

The name for this top-level directory will be `ant-just-checking-in`.

Details on the structure and architecture of `ant-just-checking-in` will be included in a separate design doc.

### Deployments

When software is updated, that new software has to be pushed into the relevant places and made live. Currently, this is done automatically by Github through the site being hosted by them. The deployments will need to emit tracking data, for the site to pick up and visualize in some way on `typesofants.org/deployments`.

The deployments will really be a glorified event-based worker. New events in the form of software updates will trigger workflows. Those workflows will replace the software, start the server, etc.

Ashley had the good idea of having a little graphic of an ant building, so the name is going to be `ant-building-projects`. That's the top-level directory, too.

Details on the structure and architecture of `ant-building-projects` will be included in a separate design doc.
