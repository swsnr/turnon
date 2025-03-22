FROM docker.io/archlinux:latest

RUN pacman -Syu --noconfirm libadwaita blueprint-compiler && \
    rm -rf /var/cache/pacman/pkg /var/lib/pacman/sync
