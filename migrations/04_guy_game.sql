CREATE TABLE `NewGuy` (
    `name` VARCHAR(255) PRIMARY KEY,
    `level` INT NOT NULL DEFAULT 0,
    `game_link` VARCHAR(255)
);
INSERT INTO `NewGuy` (`name`, `level`)
SELECT `name`,
    `level`
FROM `Guy`;
DROP TABLE `Guy`;
ALTER TABLE `NewGuy`
    RENAME TO `Guy`;