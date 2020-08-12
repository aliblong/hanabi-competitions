#!/usr/bin/env bash
script_dir="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"

source ${script_dir}/.env
sudo apt install postgresql build-essential libssl-dev pkg-config
sudo -u postgres psql -c "create database $DB_NAME"
sudo -u postgres psql -c "
    create user $DB_ADMIN_ROLE with password '$DB_ADMIN_PW';
    create user $DB_VIEWER_ROLE with password '$DB_VIEWER_PW';
    "
sudo -u postgres psql $DB_NAME -c "
    -- https://www.cybertec-postgresql.com/en/common-security-issues/
    revoke all on schema public from public;
    grant all on schema public to $DB_ADMIN_ROLE;
    grant usage on schema public to $DB_VIEWER_ROLE;
    alter default privileges for role $DB_ADMIN_ROLE in schema public grant select on tables to $DB_VIEWER_ROLE;
    alter default privileges for role $DB_ADMIN_ROLE in schema public grant select on sequences to $DB_VIEWER_ROLE;
    "
sudo -u postgres psql "postgres://${DB_ADMIN_ROLE}:${DB_ADMIN_PW}@localhost/${DB_NAME}" < schema.sql
sudo apt-get update
sudo apt-get install software-properties-common
sudo add-apt-repository ppa:certbot/certbot
sudo apt-get update
sudo apt-get install python-certbot-nginx
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
cargo install cargo-watch systemfd
