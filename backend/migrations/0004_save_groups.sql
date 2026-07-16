ALTER TABLE games ADD COLUMN IF NOT EXISTS save_group_id UUID;
UPDATE games SET save_group_id = gen_random_uuid() WHERE save_group_id IS NULL;
ALTER TABLE games ALTER COLUMN save_group_id SET DEFAULT gen_random_uuid();
ALTER TABLE games ALTER COLUMN save_group_id SET NOT NULL;

ALTER TABLE games ADD COLUMN IF NOT EXISTS save_type TEXT NOT NULL DEFAULT 'auto';

CREATE INDEX IF NOT EXISTS idx_games_save_group
    ON games (save_group_id, updated_at DESC);
