#!/bin/bash
set -u

PROGRAM_NAME=obsidian-garden
BASE_URL=https://github.com/ecarrara/obsidian-garden/releases/latest/download

message () {
  case $1 in
    error) tput setaf 1 2>/dev/null ;;
    success) tput setaf 2 2>/dev/null ;;
    warning) tput setaf 3 2>/dev/null ;;
    info) tput setaf 4 2>/dev/null ;;
  esac

  echo $2

  tput sgr0 2>/dev/null
}

exists () {
  command -v "$1" 1>/dev/null 2>&1
}

download () {
  url="$1"
  destination="$2"
  sudo="${3-}"

  if exists curl; then
    cmd="curl --silent --fail --location --output $destination $url"
  elif exists wget; then
    cmd="wget --quiet --output-document $destination $url"
  else
    message error "Unable to found curl or wget, exiting..."
    return 2
  fi

  message info "Downloading $url to $destination"
  $sudo $cmd

  if [[ "$?" -ne 0 ]]; then
    message error "Download failed."
    message warning "Only Linux x86_64 and MacOS are supported."
    exit -1
  fi
}

detect_platform () {
  platform=$(uname -s | tr '[:upper:]' '[:lower:]')
  case "$platform" in
    linux) platform="unknown-linux-musl" ;;
    darwin) platform="apple-darwin" ;;
  esac
  echo -n "$platform"
}

detect_arch () {
  arch=$(uname -m | tr '[:upper:]' '[:lower:]')
  case "$arch" in
    amd64) arch="x86_64" ;;
    arm64) arch="aarch64" ;;
  esac
  echo -n "$arch"
}


PLATFORM=$(detect_platform)
ARCH=$(detect_arch)

if [ -z "${INSTALL_DIR-}" ]; then
  INSTALL_DIR=/usr/local/bin
fi

URL=${BASE_URL}/${PROGRAM_NAME}-${ARCH}-${PLATFORM}
TARGET=${INSTALL_DIR}/${PROGRAM_NAME}

if [ -w "$INSTALL_DIR" ]; then
  download $URL $TARGET
else
  message warning "$INSTALL_DIR is not writable, sudo is required."
  download $URL $TARGET sudo
fi

message success "$PROGRAM_NAME installed."
