# API

## Migrations

You will need to install `direnv` or equivalent for environment variables

Install the migration tool with `sqlx`

```bash
cargo install sqlx-cli \
  --no-default-features \
  --features postgres,rustls,sqlite
```

- Create a new migration with `sqlx migrate add create_users`
- Migrate locally with `sqlx migrate run`
