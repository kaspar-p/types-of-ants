# Checklist

These are the things that need to happen for v1.1 to launch.

**General**

- [ ] Configure the Bazel monorepo
- [ ] Create a postgres database and allow remote connections, but not all (how to determine which?)

**ant-on-the-web**

- [ ] Self-host the site on the Raspberry Pi
  - [ ] Configure port forwarding
  - [ ] Create webserver using nginx
  - [ ] Use LetsEncrypt for HTTPS
- [ ] Create beta.typesofants.org subdomain
  - [ ] Configure the subdomain to work correctly
  - [ ] Somehow deploy the exact same site into two different places, different data.
- [ ] /blog
  - [ ] Configure the /blog path
  - [ ] Add the path into the ping tests
  - [ ] Write a first blog post about the architecture of v1.1
  - [ ] Write tests!
- [ ] /tests
  - [ ] Create the /tests path
  - [ ] Add the path into the ping tests
  - [ ] Write data-fetching code for getting each test and displaying it
  - [ ] Write tests!
- [ ] /info
  - [ ] Configure the /info path
  - [ ] Add the path into the ping tests
  - [ ] Write the content of the site
  - [ ] Write tests!
- [ ] /suggestions
  - [ ] Configure the /suggestions path
  - [ ] Add the path into the ping tests
  - [ ] Write data-fetching code to display the current suggestions
  - [ ] Write tests!
- [ ] /deployments
  - [ ] Configure the /deployments path
  - [ ] Add the path into the ping tests
  - [ ] Write data-fetching code to display the deployments by project, by status, with timestamps
  - [ ] Write tests!

**ant-just-checking-in**

- [ ] Create the package in rust
- [ ] Set up the CRON job for every 5 minutes
- [ ] Create the `curl -L <domain>` tests to make sure the HTTPS requests are 200 OK
  - [ ] Write test
  - [ ] Emit the correct data for that test
- [ ] Write more complicated tests

**ant-building-projects**

- [ ] Create the package in rust
- [ ] Create the webserver
  - [ ] Configure the /update route
- [ ] Based on the data in the database for project location, project version, etc., update the project on /update call
- [ ] Write the code to pull the new changes into a temp directory
- [ ] Write the code to build the new project binary
- [ ] Write the code to deploy the beta version of that project binary
- [ ] Write the code that waits for tests to pass in beta.
- [ ] Write the code that stops the previous project
- [ ] Write the code that copies the new binary in and starts the new project
- [ ] Emit data for each step and its relevant pieces.
