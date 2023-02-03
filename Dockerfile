# Container image that runs your code
FROM ubuntu:latest

RUN apt update && apt install curl -y
ADD https://storage.googleapis.com/devprofiler-prod/Releases/v0.1.1/linux/devprofiler_0.1.1_amd64.deb.gz /tmp
RUN gunzip /tmp/devprofiler_0.1.1_amd64.deb.gz && apt install /tmp/devprofiler_0.1.1_amd64.deb -y

# Copies your code file from your action repository to the filesystem path `/` of the container
COPY entrypoint.sh /root/entrypoint.sh
RUN chmod +x /root/entrypoint.sh
COPY /github/workspace /root/code
RUN chmod +r /root/code

# Code file to execute when the docker container starts up (`entrypoint.sh`)
ENTRYPOINT ["/root/entrypoint.sh"]