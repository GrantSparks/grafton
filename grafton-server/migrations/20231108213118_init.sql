-- Create users table.
create table if not exists users (
    id integer primary key autoincrement,
    username text not null unique,
    access_token text not null,
    refresh_token text,
    role TEXT NOT NULL DEFAULT 'None'
);
-- Create downstream_clients table.
create table if not exists downstream_clients (
    code text not null unique,
    provider text not null,
    primary key (code)
);