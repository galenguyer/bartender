CREATE TABLE drops (
    "id" SERIAL PRIMARY KEY,
    "timestamp" TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT (now() AT TIME ZONE 'UTC'),
    "username" VARCHAR(255),
    "machine" INTEGER,
    "slot" INTEGER,
    "item" INTEGER,
    "item_name" VARCHAR(255),
    "item_price" INTEGER
);
