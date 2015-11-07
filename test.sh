#!/usr/bin/env bash

set -e

testin() {
  pushd "$1"
  cargo test
  popd
}

testin common
testin client/lib
testin server/lib
