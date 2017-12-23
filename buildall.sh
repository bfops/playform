#!/bin/bash

R=0;
for file in $(find . -name 'Cargo.toml'); do
  ( echo "Building $file" && cd "$(dirname $file)" && cargo build );
  R=$?;
  if [[ $R != 0 ]]; then
    exit $R;
  fi;
done
