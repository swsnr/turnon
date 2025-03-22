FROM docker.io/archlinux:latest

RUN pacman -Syu libadwaita && \
    rm -rf /var/cache/pacman/pkg /var/lib/pacman/sync
