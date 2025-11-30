#!/bin/bash

LINUX_URL="https://cdn.kernel.org/pub/linux/kernel"
LINUX_VERSION="6.17.9"
download_linux() {
    version="${1:-$LINUX_VERSION}"
    major="${version%%.*}"
    name="linux-${version}.tar.xz"
    wget "${LINUX_URL}/v${major}.x/${name}"
    echo $name
}

compile_linux() {(
    local KDIR="${1:-${FLAKE_ROOT:-.}/linux}" 
    cd $KDIR
    [ -f ".config" ] || cp ${FLAKE_ROOT}/.config .
    make -j$(nproc)
)}

ensure_linux() {(
    local FLAKE_ROOT="${FLAKE_ROOT:-.}"
    local LOCK="${FLAKE_ROOT}/.lock"
    [ -f $LOCK ] && return 0
    touch $LOCK
    local KDIR="${FLAKE_ROOT}/linux" 
    [ -d $KDIR ] && return 0
    cd $TMP
    linuxd="$(find . -maxdepth 1 -type d -name "linux*" | head -n 1)"
    if [ -z "$linuxd" ]; then
        linuxx="$(find . -maxdepth 1 -type f -name "linux*.tar.xz" | head -n 1)"
        [ -n "$linuxx" ] || linuxx="$(download_linux)"
        tar xvf "$linuxx"
        linuxd="${linuxx%.tar.xz}"
    fi
    compile_linux "$linuxd"
    make -j$(nproc)
    [ -n "$linuxd" ] && mv $linuxd $KDIR
    rm $LOCK
)}
