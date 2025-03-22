FROM docker.io/archlinux:latest

RUN pacman -Syu --noconfirm gcc pkgconf libadwaita blueprint-compiler && \
    rm -rf /var/cache/pacman/pkg /var/lib/pacman/sync
