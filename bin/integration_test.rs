#!/usr/bin/env bash

SCRIPT_DIR=$(cd $(dirname $0); pwd)
cargo test -- --ignored; "${SCRIPT_DIR}/drop_test_dbs.sh"
