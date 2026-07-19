# API

## Migrations

### Locally

You will need to install `direnv` or equivalent for environment variables

Install the migration tool with `sqlx`

```bash
cargo install sqlx-cli \
  --no-default-features \
  --features postgres,rustls,sqlite
```

- Create a new migration with `sqlx migrate add create_users`
- Migrate locally with `sqlx migrate run`
- Update the offline SQL cache with `cargo sqlx prepare`

#### Production

Connect to the machine using its ID

```bash
fly machines list -q
fly console --machine MACHINEID
# sqlx is available on the server!
sqlx migrate run
```

#### Feature flags

Access is at `/feature_flags/web` and is password protected.
Local username/password is `admin` and `password`
