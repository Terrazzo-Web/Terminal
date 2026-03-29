FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y \
    bash \
    ca-certificates \
    curl \
    git \
    unzip \
    zip \
    build-essential \
    openjdk-21-jdk \
 && rm -rf /var/lib/apt/lists/*

RUN curl -fsSL https://github.com/bazelbuild/bazelisk/releases/latest/download/bazelisk-linux-amd64 \
    -o /usr/local/bin/bazel \
 && chmod +x /usr/local/bin/bazel

CMD ["tail", "-f", "/dev/null"]
