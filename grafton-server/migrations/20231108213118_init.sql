-- Create users table.
CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username TEXT NOT NULL UNIQUE,
    access_token TEXT NOT NULL,
    refresh_token TEXT,
    role TEXT NOT NULL DEFAULT 'None'
);
-- Create indexes for efficient search
CREATE INDEX IF NOT EXISTS idx_username ON users (username);
CREATE INDEX IF NOT EXISTS idx_access_token ON users (access_token);
CREATE INDEX IF NOT EXISTS idx_refresh_token ON users (refresh_token);
CREATE INDEX IF NOT EXISTS idx_role ON users (role);
-- Create downstream_clients table.
create table if not exists downstream_clients (
    code text not null unique,
    provider text not null,
    primary key (code)
);