# Container image that runs your code
FROM ubuntu:latest

RUN apt update && apt install curl -y
ADD https://storage.googleapis.com/devprofiler-prod/Releases/v0.1.1/linux/devprofiler_v0.1.1.deb.gz /tmp
RUN gunzip /tmp/devprofiler_v0.1.1.deb.gz && apt install /tmp/devprofiler_v0.1.1.deb -y

# Copies your code file from your action repository to the filesystem path `/` of the container
COPY entrypoint.sh /entrypoint.sh
RUN chmod +x entrypoint.sh

# Code file to execute when the docker container starts up (`entrypoint.sh`)
ENTRYPOINT ["/entrypoint.sh"]