#!/usr/bin/env bash
script_dir="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"

source ${script_dir}/.env
sudo apt install postgresql
sudo -u postgres psql -c "
    create database $DB_NAME;
    create user $DB_ADMIN_ROLE with password '$DB_ADMIN_PW';
    create user $DB_VIEWER_ROLE with password '$DB_VIEWER_PW';
    "
sudo -u postgres psql $DB_NAME -c "
    -- https://www.cybertec-postgresql.com/en/common-security-issues/
    revoke all on schema public from public;
    alter default privileges in schema public grant select on tables to $DB_VIEWER_ROLE;
    alter default privileges in schema public grant select on sequences to $DB_VIEWER_ROLE;
    alter default privileges in schema public grant all on tables to $DB_ADMIN_ROLE;
    alter default privileges in schema public grant all on sequences to $DB_ADMIN_ROLE;
    "
sudo -u postgres psql --host=localhost --dbname=$DB_NAME --username=$DB_ADMIN_PW < schema.sql
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
cargo install cargo-watch systemfd
