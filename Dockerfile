FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y \
    bash \
    ca-certificates \
    curl \
    git \
    openssh-server \
    unzip \
    zip \
    build-essential \
    openjdk-21-jdk \
 && rm -rf /var/lib/apt/lists/*

RUN id -u ubuntu >/dev/null 2>&1 || useradd --create-home --shell /bin/bash ubuntu \
 && mkdir -p /run/sshd \
 && sed -i 's/^#\?PasswordAuthentication .*/PasswordAuthentication no/' /etc/ssh/sshd_config \
 && sed -i 's/^#\?PermitRootLogin .*/PermitRootLogin no/' /etc/ssh/sshd_config \
 && (grep -qxF 'AllowUsers ubuntu' /etc/ssh/sshd_config || printf '\nAllowUsers ubuntu\n' >> /etc/ssh/sshd_config)

RUN curl -fsSL https://github.com/bazelbuild/bazelisk/releases/latest/download/bazelisk-linux-amd64 \
    -o /usr/local/bin/bazel \
 && chmod +x /usr/local/bin/bazel

CMD ["tail", "-f", "/dev/null"]
