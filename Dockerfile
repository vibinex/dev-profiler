# Use a lightweight Linux distribution as the base image
FROM alpine:latest

# Install dependencies required by the application
RUN apk update && \
    apk add git openssh openssl-dev build-base cargo && \
    rm -rf /var/cache/apk/*

# Set up SSH for Git access
COPY id_rsa /root/.ssh/id_rsa
RUN chmod 600 /root/.ssh/id_rsa && \
    echo "StrictHostKeyChecking no" >> /etc/ssh/ssh_config

# Set environment variables
ENV GIT_USERNAME=""
ENV GIT_PASSWORD=""
ENV REPO_OWNER=""

# Copy the Rust application code into the container
COPY ./devprofiler /app/devprofiler
COPY ./pubsub-sa.json /app/pubsub-sa.json
WORKDIR /app/devprofiler

# Build the Rust application
RUN cargo build --release

# Start the Rust application
CMD ["target/release/devprofiler"]
