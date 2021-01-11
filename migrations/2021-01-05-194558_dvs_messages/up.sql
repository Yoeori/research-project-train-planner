-- DVS Messages
-- Stores the received messages as plain text
-- Helpfull for future (re)-interpretation of the messages

CREATE TABLE dvs_messages (
    id INT NOT NULL PRIMARY KEY AUTO_INCREMENT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    envelope VARCHAR(255),
    message TEXT
);