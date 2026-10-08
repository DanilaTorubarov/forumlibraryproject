# Rust, React, and PostgreSQL with Docker

The `app` container runs the Rust server on port 8080 and serves the built React frontend. The `db` container runs PostgreSQL 18. PostgreSQL data is stored in the `pgdata` Docker volume, separately from the PostgreSQL installation on your Mac.

## Before starting

Start Docker Desktop and sign in to Docker Hardened Images:

```sh
docker login dhi.io
```

On a fresh clone, copy the example file, then edit `.env.docker` to use your own password in both places:

```sh
cp -n .env.docker.example .env.docker
```

The file contains these two values:

```dotenv
POSTGRES_PASSWORD=replace_with_a_private_password
DATABASE_URL=postgresql://postgres:replace_with_a_private_password@db:5432/project_db
```

`db` is the PostgreSQL container's hostname inside Compose. If your password contains URL-special characters, URL-encode them in `DATABASE_URL`. Do not commit `.env.docker`.

## Build and start

Run these commands from this project directory:

```sh
docker compose --env-file .env.docker up --build -d
docker compose --env-file .env.docker ps
```

Open <http://127.0.0.1:8080>. To see startup messages:

```sh
docker compose --env-file .env.docker logs -f app db
```

Press `Ctrl+C` to stop following logs; the containers keep running.

## Set up the database table

This container starts with an empty `project_db`; it does not copy tables or users from PostgreSQL on your Mac. Open `psql` inside the database container:

```sh
docker compose --env-file .env.docker exec db psql -U postgres -d project_db
```

If you have not created the registration table in this database, run this SQL at the `psql` prompt:

```sql
CREATE TABLE IF NOT EXISTS users (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    name TEXT NOT NULL,
    surname TEXT NOT NULL,
    login TEXT NOT NULL UNIQUE,
    email TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL
);
```

Use `\dt` to list tables and `\q` to leave `psql`.

## Everyday commands

```sh
# Start again and rebuild after code changes
docker compose --env-file .env.docker up --build -d

# Check container status
docker compose --env-file .env.docker ps

# Follow app logs
docker compose --env-file .env.docker logs -f app

# Stop and remove the containers; keep the database volume
docker compose --env-file .env.docker down
```

`docker compose --env-file .env.docker down -v` also **deletes the database volume and its data**. Use it only when you want a fresh database.
