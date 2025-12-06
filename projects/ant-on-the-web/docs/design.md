# Webserver

The webserver has two functions: serve the webpage(s), and serve the data
associated with the webpages.

## Why not split them into different servers?

While there are some advantages to splitting them, since they are mostly
different functions, there are also disadvantages. The largest disadvantage is
that with two different servers (and no caching), a request for the webpage
automatically becomes a serial request to both servers

1. Request for webpage
2. Webserver request for data
3. Webserver gets data
4. User gets webpage

With a single server, this is two steps:

1. Request for webpage
2. User gets webpage
