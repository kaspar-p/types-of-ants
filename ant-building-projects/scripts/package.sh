#!/bin/bash

echo 'Building deployment artifacts!'

cd $ANTHILL_ROOT
./ant-building-projects/scripts/package-ant-on-the-web.sh
