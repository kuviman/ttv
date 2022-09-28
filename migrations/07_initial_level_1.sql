CREATE TABLE `NewGuy` (
    `name` VARCHAR(255) PRIMARY KEY,
    `level` INT NOT NULL DEFAULT 1,
    `game_link` VARCHAR(255) DEFAULT NULL,
    `game_played` BOOLEAN NOT NULL DEFAULT FALSE,
    `hat` VARCHAR(32),
    `robe` VARCHAR(32),
    `face` VARCHAR(32),
    `beard` VARCHAR(32),
    `custom_skin` VARCHAR(32) DEFAULT NULL,
    `outfit_color` VARCHAR(32)
);
INSERT INTO `NewGuy` (
        `name`,
        `level`,
        `game_link`,
        `game_played`,
        `hat`,
        `robe`,
        `face`,
        `beard`,
        `custom_skin`,
        `outfit_color`
    )
SELECT `name`,
    `level`,
    `game_link`,
    `game_played`,
    `hat`,
    `robe`,
    `face`,
    `beard`,
    `custom_skin`,
    `outfit_color`
FROM `Guy`;
DROP TABLE `Guy`;
ALTER TABLE `NewGuy`
    RENAME TO `Guy`;