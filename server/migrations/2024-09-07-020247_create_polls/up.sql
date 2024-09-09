-- Your SQL goes here
CREATE TABLE IF NOT EXISTS Users (
    id UUID PRIMARY KEY DEFAULT GEN_RANDOM_UUID(),
    display_name VARCHAR(100) NOT NULL
);

CREATE TABLE IF NOT EXISTS Polls (
    id UUID PRIMARY KEY DEFAULT GEN_RANDOM_UUID(),
    title VARCHAR(300) NOT NULL,
    winner_count INTEGER NOT NULL DEFAULT 1,
    write_ins_allowed BOOLEAN NOT NULL DEFAULT FALSE,
    close_after_time TIMESTAMP,
    close_after_votes INTEGER,
    owner_id UUID NOT NULL,
    created_at TIMESTAMP NOT NULL,
    closed_at TIMESTAMP,
    FOREIGN KEY (owner_id) REFERENCES Users (id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS PollOptions (
    poll_id UUID NOT NULL,
    id INTEGER NOT NULL,
    description VARCHAR(300) NOT NULL,
    PRIMARY KEY (poll_id, id),
    FOREIGN KEY (poll_id) REFERENCES Polls (id) ON DELETE CASCADE
);
