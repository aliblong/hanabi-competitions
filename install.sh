#!/usr/bin/env bash

# This is not really meant to be a one-shot install script so much as a general guideline
# on how to stand up the application.

script_dir="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"

# db setup
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

# install rustup (rust toolchain)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# for local dev, you probably want these, which empower recompilation-on-file-modification
# with minimal interruption to the server
cargo install cargo-watch systemfd

# setting up certbot with a nginx reverse proxy on Ubuntu 20.04
# I don't include the nginx or systemd configs, since they're basically an out-of-the-box
# solution for a reverse proxy accepting https traffic only.
# You'll want to run `cargo build --release` and point the systemd config at
# target/release/hanabi-competitions.
sudo apt-get update
sudo apt-get install nginx software-properties-common
sudo add-apt-repository ppa:certbot/certbot
sudo apt-get update
sudo apt-get install python-certbot-nginx
# sudo certbot --nginx
