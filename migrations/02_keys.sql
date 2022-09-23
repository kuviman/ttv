-- sqlite does not support significant changes to tables
-- like adding keys or inserting a column in specific position
-- https://sqlite.org/faq.html#q11
-- so instead we should create new table and copy the data
CREATE TABLE `Guy` (
    `name` VARCHAR(255) PRIMARY KEY,
    `level` INT
);
INSERT INTO `Guy`
SELECT *
FROM `Persons`;