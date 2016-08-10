true;
while [[ $? == 0 ]]; do
  while [[ $? == 0 ]]; do
    clear;
    echo "Build unchanged? Checking again..";
    cargo build;
  done
  build-client.sh &&
  clear &&
  cargo build --release &&
  rm -f default.terrain
done
