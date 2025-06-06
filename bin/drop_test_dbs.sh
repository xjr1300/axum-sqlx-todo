#!/usr/bin/env bash

set -x
set -eo pipefail

DB_USER="${POSTGRES_DATABASE__USER:=todo}"
DB_PASSWORD="${POSTGRES_DATABASE__PASSWORD:=todo-password}"
DB_PORT="${POSTGRES_DATABASE__PORT:=5432}"
DB_HOST="${POSTGRES_DATABASE__HOST:=localhost}"

TEST_DB_PREFIX="test_todo_db_"

# 統合テスト用のデータベースを取得
export PGPASSWORD="${DB_PASSWORD}"
mapfile -t TEST_DBS < <(psql -h "${DB_HOST}" -p "${DB_PORT}" -U "${DB_USER}" -d postgres -P "pager=off" -c "\l" | grep "${TEST_DB_PREFIX}" | cut -d "|" -f 1 | tr -d ' ')

for TEST_DB in "${TEST_DBS[@]}"; do
    echo "Dropping database: ${TEST_DB}"
done

# 統合テスト用のデータベースを削除
for TEST_DB in "${TEST_DBS[@]}"; do
    psql -h "${DB_HOST}" -p "${DB_PORT}" -U "${DB_USER}" -d postgres -c "DROP DATABASE ${TEST_DB};"
done
