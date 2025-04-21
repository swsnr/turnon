FROM docker.io/fedora:42
LABEL org.opencontainers.image.description "CI image for de.swsnr.turnon"

RUN dnf install -y --setopt=install_weak_deps=False blueprint-compiler libadwaita-devel gcc pkgconf git gettext make appstream && \
    dnf clean all && rm -rf /var/cache/yum
