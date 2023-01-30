# Container image that runs your code
FROM ubuntu:latest

RUN apt update && apt install curl && curl https://storage.googleapis.com/devprofiler-prod/Releases/v0.1.0/linux/devprofiler_0.1.0_amd64.deb.gz
RUN apt install devprofiler_0.1.0_amd64.deb.gz

# Copies your code file from your action repository to the filesystem path `/` of the container
COPY entrypoint.sh /entrypoint.sh

# Code file to execute when the docker container starts up (`entrypoint.sh`)
ENTRYPOINT ["/entrypoint.sh"]