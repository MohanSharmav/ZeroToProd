#!/usr/bin/env bash
set -x
set -eo pipefail

if ! [ -x "$(command -v psql)" ]; then
  echo >&2 "Error: psql is not installed."
  exit 1
fi

if ! [ -x "$(command -v sqlx)" ]; then
  echo >&2 "Error: sqlx is not installed."
  echo >&2 "Use:"
  echo >&2 "    cargo install --version='~0.6' sqlx-cli \
--no-default-features --features rustls,postgres"
  echo >&2 "to install it."
  exit 1
fi


DB_USER=${POSTGRES_USER:=mohanvenkatesh}
DB_PASSWORD="${POSTGRES_PASSWORD:=Msvmsd183!}"
DB_NAME="${POSTGRES_DB:=mohan}"
DB_PORT="${POSTGRES_PORT:=5432}"
DB_HOST="${POSTGRES_HOST:=localhost}"

if [[ -z "${SKIP_DOCKER}" ]]
then
  docker run \
  -e POSTGRES_USER=${DB_USER} \
  -e POSTGRES_PASSWORD=${DB_PASSWORD} \
  -e POSTGRES_DB=${DB_NAME}\
  -p "${DB_PORT}":5432\
  -d postgres\
  postgres -N 1000
fi

export PGPASSWORD="${DB_PASSWORD}"
until psql -h "${DB_HOST}" -U "${DB_USER}" -p "${DB_PORT}" -d "postgres" -c '\q'; do
>&2 echo "Postgres is still unavailable - sleeping"
sleep 1
done
>&2 echo "Postgres is up and running on port ${DB_PORT} - running migrations now!"
DATABASE_URL=postgres://${DB_USER}:${DB_PASSWORD}@${DB_HOST}

export DATABASE_URL
sqlx database create
#sqlx database create
sqlx migrate run
#sqlx migrate add create_subscriptions_table

#>&2 echo "Postgres has been migrated, ready to go!"