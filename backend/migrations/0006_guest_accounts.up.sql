-- Ten shared guest accounts, Guest and Guest2 to Guest10, for playing without signing up. A
-- guest is free while it has no live session; signing in as a guest takes the first free one.
-- They have no password: password sign-in never considers them.
ALTER TABLE account.users ADD COLUMN guest_number integer;

ALTER TABLE account.users
ADD CONSTRAINT users_guest_number_check CHECK (guest_number BETWEEN 1 AND 10),
ADD CONSTRAINT users_guest_number_key UNIQUE (guest_number);

INSERT INTO account.users (id, email, username, password_hash, guest_number)
SELECT
    gen_random_uuid(),
    'guest' || number || '@guest.invalid',
    CASE WHEN number = 1 THEN 'Guest' ELSE 'Guest' || number END,
    '!',
    number
FROM generate_series(1, 10) AS number;
