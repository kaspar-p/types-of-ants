#!/bin/bash

echo 'Building deployment artifacts!'

if [[ -z $ANTHILL_ROOT ]]; then
  echo "No \$ANTHILL_ROOT variable defined! Needs to be the root of the git repository!"
  exit 1
fi

$ANTHILL_ROOT/ant-building-projects/scripts/package-ant-on-the-web.sh
