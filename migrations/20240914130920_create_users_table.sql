-- Add migration script here
Create Table users(
    user_id uuid Primary key,
    username Text Not Null Unique,
    password Text Not Null
);
