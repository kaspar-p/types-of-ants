#!/bin/sh

echo 'STARTING SSL RENEWAL AGENT'

crond --help

echo "* * * * * echo 'I love running my crons'" >> /etc/crontabs/root
echo "" >> /etc/crontabs/root

cat /etc/crontabs/root

crond -l 0 -f -c /etc/crontabs/root > /proc/1/fd/1 2> /proc/1/fd/2 &
