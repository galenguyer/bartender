CREATE TABLE machines (
    "id" SERIAL PRIMARY KEY,
    "name" VARCHAR(255) NOT NULL,
    "display_name" VARCHAR(255) NOT NULL,
    "active" BOOLEAN NOT NULL DEFAULT true
);

CREATE TABLE items (
    "id" SERIAL PRIMARY KEY,
    "name" VARCHAR(255) NOT NULL,
    "price" INTEGER
);

CREATE TABLE slots (
    "machine" INTEGER NOT NULL,
    "number" INTEGER NOT NULL,
    "item" INTEGER,
    "active" BOOLEAN NOT NULL DEFAULT false,
    "count" INTEGER,
    PRIMARY KEY (machine, "number"),
    CONSTRAINT fk_machine
        FOREIGN KEY (machine)
        REFERENCES machines(id)
        ON DELETE CASCADE,
    CONSTRAINT fk_item
        FOREIGN KEY (item)
        REFERENCES items(id)
        ON DELETE CASCADE
);
