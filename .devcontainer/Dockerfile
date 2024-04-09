# [Choice] Debian OS version (use bookworm, or bullseye on local arm64/Apple Silicon): bookworm, buster, bullseye
ARG VARIANT="bookworm"
FROM rust:1-${VARIANT}

RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
    # Remove imagemagick due to https://security-tracker.debian.org/tracker/CVE-2019-10131
    && apt-get purge -y imagemagick imagemagick-6-common

# Additional OS packages.
RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
    && apt-get -y install --no-install-recommends protobuf-compiler \
    && apt-get -y install poppler-utils \
    && apt-get -y install cmake

# packages required by wkhtmlto*:
RUN apt-get install -y \
    xfonts-base \
    xfonts-75dpi \
    pdftk \
    fontconfig

RUN curl -L#o wkhtmltopdf.deb https://github.com/wkhtmltopdf/packaging/releases/download/0.12.6.1-3/wkhtmltox_0.12.6.1-3.bookworm_amd64.deb
RUN dpkg -i wkhtmltopdf.deb; apt-get install -y -f

# Remember to clean your package manager cache to reduce your custom image size...
RUN apt-get clean all \
    && rm -rvf /var/lib/apt/lists/*