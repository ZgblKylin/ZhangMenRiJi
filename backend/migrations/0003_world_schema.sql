-- 《掌门日记》v3：世界快照与可查询事件元数据。
ALTER TABLE games ADD COLUMN IF NOT EXISTS schema_version INT NOT NULL DEFAULT 3;

CREATE TABLE IF NOT EXISTS game_snapshots (
    game_id       UUID NOT NULL,
    turn          INT NOT NULL,
    state         JSONB NOT NULL,
    checksum      TEXT NOT NULL DEFAULT '',
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (game_id, turn)
);

ALTER TABLE events ADD COLUMN IF NOT EXISTS category TEXT NOT NULL DEFAULT 'world';
ALTER TABLE events ADD COLUMN IF NOT EXISTS payload JSONB NOT NULL DEFAULT '{}'::jsonb;

CREATE INDEX IF NOT EXISTS idx_snapshots_game_turn
    ON game_snapshots (game_id, turn DESC);
CREATE INDEX IF NOT EXISTS idx_events_category
    ON events (game_id, category, year DESC, month DESC);
