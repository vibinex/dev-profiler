#!/bin/sh -l
cd /home/runner/work/dev-profiler/
ls

devprofiler -- docker

timestamp=$(date +%s)
filename="${timestamp}-devprofile.jsonl.gz"
mv devprofile.jsonl.gz "${filename}"

curl -F "file=@${filename}"  https://gcscruncsql-k7jns52mtq-el.a.run.app/upload