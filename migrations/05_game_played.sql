CREATE TABLE `NewGuy` (
    `name` VARCHAR(255) PRIMARY KEY,
    `level` INT NOT NULL DEFAULT 0,
    `game_link` VARCHAR(255) DEFAULT NULL,
    `game_played` BOOLEAN NOT NULL DEFAULT FALSE
);
INSERT INTO `NewGuy` (`name`, `level`, `game_link`)
SELECT `name`,
    `level`,
    `game_link`
FROM `Guy`;
DROP TABLE `Guy`;
ALTER TABLE `NewGuy`
    RENAME TO `Guy`;