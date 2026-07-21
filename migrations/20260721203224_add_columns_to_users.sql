-- Add migration script here

ALTER TABLE users ADD COLUMN updated_at DATETIME NOT NULL;
ALTER TABLE users ADD COLUMN created_at DATETIME NOT NULL;
