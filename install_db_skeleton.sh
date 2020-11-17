#!/usr/bin/env bash

# This is meant for building the DB from a dump

script_dir="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"

# db setup
source ${script_dir}/.env
sudo apt install postgresql build-essential libssl-dev pkg-config
sudo -u postgres psql -c "create database $DB_NAME"
sudo -u postgres psql -c "
    create user $DB_ADMIN_ROLE with password '$DB_ADMIN_PW';
    create user $DB_VIEWER_ROLE with password '$DB_VIEWER_PW';
    "
sudo -u postgres psql $DB_NAME < $DB_DUMP
