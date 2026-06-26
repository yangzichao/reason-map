-- v2: switch FTS to the trigram tokenizer so CJK substring search works (the app is
-- Chinese-first; the default unicode61 tokenizer does not segment Chinese). Also add an
-- idempotency marker so a conceded/rebutted challenge can be promoted to a node only once.

DROP TRIGGER IF EXISTS nodes_fts_ai;
DROP TRIGGER IF EXISTS nodes_fts_ad;
DROP TRIGGER IF EXISTS nodes_fts_au;
DROP TABLE IF EXISTS nodes_fts;

CREATE VIRTUAL TABLE nodes_fts USING fts5(node_id UNINDEXED, text, tokenize = 'trigram');
INSERT INTO nodes_fts(node_id, text)
    SELECT id, text FROM nodes WHERE deleted_at IS NULL;

CREATE TRIGGER nodes_fts_ai AFTER INSERT ON nodes BEGIN
    INSERT INTO nodes_fts(node_id, text) VALUES (new.id, new.text);
END;
CREATE TRIGGER nodes_fts_ad AFTER DELETE ON nodes BEGIN
    DELETE FROM nodes_fts WHERE node_id = old.id;
END;
CREATE TRIGGER nodes_fts_au AFTER UPDATE OF text ON nodes BEGIN
    DELETE FROM nodes_fts WHERE node_id = old.id;
    INSERT INTO nodes_fts(node_id, text) VALUES (new.id, new.text);
END;
-- Soft-delete should also drop the row from the FTS index (nit.1).
CREATE TRIGGER nodes_fts_soft_delete AFTER UPDATE OF deleted_at ON nodes
    WHEN new.deleted_at IS NOT NULL BEGIN
    DELETE FROM nodes_fts WHERE node_id = old.id;
END;

ALTER TABLE challenges ADD COLUMN promoted_node_id TEXT;
