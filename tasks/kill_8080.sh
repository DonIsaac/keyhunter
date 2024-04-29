#!/bin/bash
# finds the PID of the server listening on port 8080 and kills it

set -e

to_parse=$(lsof -i -n -P | grep 8080)

if [ -z "$to_parse" ]; then
    echo "No process listening on port 8080"
    exit 0
fi

pid=$(echo $to_parse | awk '{print $2}')
echo "Killing process $pid"
kill $pid
