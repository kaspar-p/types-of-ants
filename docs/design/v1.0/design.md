# Design of typesofants.org

I want to transform typesofants.org into more of an interactive developing website. I want everything that normally goes into developing a service to be visible to everyone.

## Goals

The goals are put here for two reasons: as an excuse to learn the technology, and as a fun thing to showcase for the future.

There are many processes that businesses running to make money would never want their customers (or competing businesses) to see. These include things like deployment schedules for software, versioning info, architecture diagrams and documents, currently passing or failing tests, the number of servers they have and their status, and others. 

The goal of v1.0 is for typesofants.org to be a full end-to-end service, but show all of those things off, as a part of the development process. The parts that make that up are:

- Somewhere for the user to see submitted suggestions
- The passing/failing tests for everything (main site, database, etc.)
- The current software deployments
- A blog
- Contact me information
- Somewhere for the users to see how many machines are doing what. How many load balancers, how many web nodes, and more. They should also be able to download an executable that allows users to become a web node for typesofants.org, if they want.
- Allow the users to run SQL queries against a read-only version of the database.

## High-level Architecture

The architecture of typesofants.org will be very different to what it is now. Currently, it's a single static HTML site, hosted through Github Pages.

There will be three major pieces to typesofants.org. The main website, some monitoring software, and some deployment software. The website will be the same, having paths and pages for most of the goals mentioned above. The main website will be self-hosted.

One of the larger pieces missing in the current architecture, though, is any sort of continuous monitoring and testing. There will be a process that is essentially just a CRON job, running in an interval. Every time it runs, it's going to run a suite of integration tests on the site. Each test will emit some pass/fail data (along with other data), and that data can be used to determine if the site is up or not, or which features aren't working right.

Another piece of software that will be useful once the website is self-hosted is some deployment/building software. Essentially, this is just a listener that waits for new versions of projects to be pushed. Once new versions **are** available, it builds those new versions into their corresponding binaries, stops the current process, and restarts the process with the new binary.

This continuous integration will allow automated changes to reach production, without very much manual testing.

Though a load-balancer may be helpful in the future, it is omitted because typesofants.org isn't that big, yet.

## Details

### Repository

Everything regarding this project will stay in this Github repository, exactly the same. The structure of the repo will change dramatically. There are some separate packages, each will be a top-level directory.

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

### Load Balancing

I'm unsure if this is a part of the website or not. It seems to me like it isn't, and that in a best-case scenario, the load balancers are different, CPU-optimized machines, while the webservers can be anything. 

The name of this service is not yet known, candidates are:
- `ant-balancing`, `balancing-ant`
- `ant-with-first-contact`, `first-contact-ant`, `doorman-ant`
- `ant-routing`, `ant-rerouting`

Details of the structure and architecture of the load balancing software will be included in a separate design doc.
